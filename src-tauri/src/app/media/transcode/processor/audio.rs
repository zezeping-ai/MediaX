use super::progress::update_progress;
use super::JobResult;
use crate::app::media::playback::audio_shared::fallback_channel_layout;
use crate::app::media::transcode::state::{TranscodeJob, TranscodeState};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::filter;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::media::Type;
use ffmpeg_next::Packet;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager};

pub(super) fn transcode_audio_with_progress(
    app: &AppHandle,
    job: &TranscodeJob,
    output_path: &Path,
    rate: f64,
    format_name: &str,
) -> Result<JobResult, String> {
    let mut input_ctx = format::input(&job.source_path).map_err(|err| format!("打开输入失败: {err}"))?;
    let duration_ms = {
        let d = input_ctx.duration();
        if d > 0 {
            Some((d as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE) * 1000.0) as u64)
        } else {
            None
        }
    };
    let Some(audio_stream) = input_ctx.streams().best(Type::Audio) else {
        return Err("未找到音频流".to_string());
    };
    let audio_stream_index = audio_stream.index();
    let mut decoder = ffmpeg_next::codec::context::Context::from_parameters(audio_stream.parameters())
        .map_err(|err| format!("创建音频解码上下文失败: {err}"))?
        .decoder()
        .audio()
        .map_err(|err| format!("打开音频解码器失败: {err}"))?;

    let mut output_ctx = format::output(output_path).map_err(|err| format!("打开输出失败: {err}"))?;
    let global_header = output_ctx.format().flags().contains(format::Flags::GLOBAL_HEADER);
    let encoder_codec_id = parse_audio_codec_id(format_name);
    let encoder_codec_opt = ffmpeg_next::encoder::find(encoder_codec_id);
    let mut out_stream = output_ctx
        .add_stream(encoder_codec_opt)
        .map_err(|err| format!("创建音频输出流失败: {err}"))?;
    let out_stream_index = out_stream.index();
    let encoder_codec = encoder_codec_opt.ok_or_else(|| "找不到可用音频编码器".to_string())?;
    let audio_codec = encoder_codec
        .audio()
        .map_err(|err| format!("获取音频编码器信息失败: {err}"))?;
    let mut encoder = ffmpeg_next::codec::context::Context::from_parameters(out_stream.parameters())
        .map_err(|err| format!("创建音频编码上下文失败: {err}"))?
        .encoder()
        .audio()
        .map_err(|err| format!("创建音频编码器失败: {err}"))?;
    let channel_layout = audio_codec
        .channel_layouts()
        .map(|layouts| layouts.best(fallback_channel_layout(&decoder).channels()))
        .unwrap_or_else(|| fallback_channel_layout(&decoder));
    if global_header {
        encoder.set_flags(codec::Flags::GLOBAL_HEADER);
    }
    encoder.set_rate(decoder.rate() as i32);
    encoder.set_channel_layout(channel_layout);
    encoder.set_format(
        audio_codec
            .formats()
            .and_then(|mut f| f.next())
            .ok_or_else(|| "音频编码器不支持输出格式".to_string())?,
    );
    encoder.set_time_base((1, decoder.rate() as i32));
    out_stream.set_time_base((1, decoder.rate() as i32));
    let mut opened_encoder = encoder
        .open_as(encoder_codec)
        .map_err(|err| format!("打开音频编码器失败: {err}"))?;
    out_stream.set_parameters(&opened_encoder);
    drop(out_stream);

    let filter_spec = build_atempo_filter_chain(rate);
    let mut filter_graph = build_audio_filter_graph(&filter_spec, &decoder, &opened_encoder)?;
    output_ctx
        .write_header()
        .map_err(|err| format!("写入输出头失败: {err}"))?;
    let out_time_base = output_ctx
        .stream(out_stream_index)
        .ok_or_else(|| "输出音频流不存在".to_string())?
        .time_base();

    let mut decoded = frame::Audio::empty();
    let mut filtered = frame::Audio::empty();
    let mut encoded = Packet::empty();
    let mut last_percent = 0.0_f64;

    for (stream, mut packet) in input_ctx.packets() {
        let canceled = {
            let state = app.state::<TranscodeState>();
            state.with_inner(|inner| inner.canceled.contains(&job.id))
        };
        if canceled {
            let _ = output_ctx.write_trailer();
            return Ok(JobResult::Skipped(
                fs::metadata(output_path).map(|m| m.len()).unwrap_or(0),
                "任务已取消".to_string(),
            ));
        }
        if stream.index() != audio_stream_index {
            continue;
        }
        packet.rescale_ts(stream.time_base(), decoder.time_base());
        decoder
            .send_packet(&packet)
            .map_err(|err| format!("向音频解码器送包失败: {err}"))?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            let ts = decoded.timestamp();
            decoded.set_pts(ts);
            filter_graph
                .get("in")
                .ok_or_else(|| "获取音频滤镜输入失败".to_string())?
                .source()
                .add(&decoded)
                .map_err(|err| format!("推送音频帧到滤镜失败: {err}"))?;
            while filter_graph
                .get("out")
                .ok_or_else(|| "获取音频滤镜输出失败".to_string())?
                .sink()
                .frame(&mut filtered)
                .is_ok()
            {
                opened_encoder
                    .send_frame(&filtered)
                    .map_err(|err| format!("向音频编码器送帧失败: {err}"))?;
                while opened_encoder.receive_packet(&mut encoded).is_ok() {
                    encoded.set_stream(out_stream_index);
                    encoded.rescale_ts(decoder.time_base(), out_time_base);
                    encoded
                        .write_interleaved(&mut output_ctx)
                        .map_err(|err| format!("写入音频输出包失败: {err}"))?;
                }
            }
        }
        if let (Some(total_ms), Some(pts)) = (duration_ms, packet.pts()) {
            let pts_ms = (pts as f64 * f64::from(stream.time_base()) * 1000.0).max(0.0);
            let mut percent = (pts_ms / total_ms as f64) * 100.0;
            percent = percent.clamp(0.0, 99.0);
            if percent + 0.2 >= last_percent {
                last_percent = percent;
                let size = fs::metadata(output_path).map(|m| m.len()).ok();
                update_progress(app, job.id, percent, size, None, None);
            }
        }
    }

    decoder
        .send_eof()
        .map_err(|err| format!("发送音频解码 EOF 失败: {err}"))?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        let ts = decoded.timestamp();
        decoded.set_pts(ts);
        filter_graph
            .get("in")
            .ok_or_else(|| "获取音频滤镜输入失败".to_string())?
            .source()
            .add(&decoded)
            .map_err(|err| format!("推送音频帧到滤镜失败: {err}"))?;
        while filter_graph
            .get("out")
            .ok_or_else(|| "获取音频滤镜输出失败".to_string())?
            .sink()
            .frame(&mut filtered)
            .is_ok()
        {
            opened_encoder
                .send_frame(&filtered)
                .map_err(|err| format!("向音频编码器送帧失败: {err}"))?;
            while opened_encoder.receive_packet(&mut encoded).is_ok() {
                encoded.set_stream(out_stream_index);
                encoded.rescale_ts(decoder.time_base(), out_time_base);
                encoded
                    .write_interleaved(&mut output_ctx)
                    .map_err(|err| format!("写入音频输出包失败: {err}"))?;
            }
        }
    }
    filter_graph
        .get("in")
        .ok_or_else(|| "获取音频滤镜输入失败".to_string())?
        .source()
        .flush()
        .map_err(|err| format!("刷新音频滤镜失败: {err}"))?;
    while filter_graph
        .get("out")
        .ok_or_else(|| "获取音频滤镜输出失败".to_string())?
        .sink()
        .frame(&mut filtered)
        .is_ok()
    {
        opened_encoder
            .send_frame(&filtered)
            .map_err(|err| format!("向音频编码器送帧失败: {err}"))?;
        while opened_encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(out_stream_index);
            encoded.rescale_ts(decoder.time_base(), out_time_base);
            encoded
                .write_interleaved(&mut output_ctx)
                .map_err(|err| format!("写入音频输出包失败: {err}"))?;
        }
    }
    opened_encoder
        .send_eof()
        .map_err(|err| format!("发送音频编码 EOF 失败: {err}"))?;
    while opened_encoder.receive_packet(&mut encoded).is_ok() {
        encoded.set_stream(out_stream_index);
        encoded.rescale_ts(decoder.time_base(), out_time_base);
        encoded
            .write_interleaved(&mut output_ctx)
            .map_err(|err| format!("写入音频输出包失败: {err}"))?;
    }
    output_ctx
        .write_trailer()
        .map_err(|err| format!("写入输出尾失败: {err}"))?;
    let final_size = fs::metadata(output_path)
        .map(|m| m.len())
        .map_err(|err| format!("读取输出文件元数据失败: {err}"))?;
    Ok(JobResult::Success(final_size))
}

fn parse_audio_codec_id(value: &str) -> codec::Id {
    let trimmed = value.trim().to_lowercase();
    let codec_name = trimmed
        .split('/')
        .nth(1)
        .unwrap_or("aac")
        .split('+')
        .next()
        .unwrap_or("aac");
    match codec_name {
        "mp3" | "libmp3lame" => codec::Id::MP3,
        "opus" | "libopus" => codec::Id::OPUS,
        "mp2" => codec::Id::MP2,
        "wmav2" => codec::Id::WMAV2,
        _ => codec::Id::AAC,
    }
}

fn build_audio_filter_graph(
    filter_spec: &str,
    decoder: &ffmpeg_next::decoder::Audio,
    encoder: &ffmpeg_next::encoder::Audio,
) -> Result<filter::Graph, String> {
    let mut graph = filter::Graph::new();
    let input_layout = fallback_channel_layout(decoder);
    let channel_layout_bits = input_layout.bits();
    let args = if channel_layout_bits == 0 {
        format!(
            "time_base={}:sample_rate={}:sample_fmt={}:channels={}",
            decoder.time_base(),
            decoder.rate(),
            decoder.format().name(),
            input_layout.channels()
        )
    } else {
        format!(
            "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
            decoder.time_base(),
            decoder.rate(),
            decoder.format().name(),
            channel_layout_bits
        )
    };
    graph
        .add(
            &filter::find("abuffer").ok_or_else(|| "找不到音频输入滤镜".to_string())?,
            "in",
            &args,
        )
        .map_err(|err| format!("创建音频输入滤镜失败: {err}"))?;
    graph
        .add(
            &filter::find("abuffersink").ok_or_else(|| "找不到音频输出滤镜".to_string())?,
            "out",
            "",
        )
        .map_err(|err| format!("创建音频输出滤镜失败: {err}"))?;
    {
        let mut out = graph
            .get("out")
            .ok_or_else(|| "获取音频输出滤镜失败".to_string())?;
        out.set_sample_format(encoder.format());
        out.set_channel_layout(encoder.channel_layout());
        out.set_sample_rate(encoder.rate());
    }
    graph
        .output("in", 0)
        .and_then(|p| p.input("out", 0))
        .and_then(|p| p.parse(filter_spec))
        .map_err(|err| format!("解析音频滤镜失败: {err}"))?;
    graph
        .validate()
        .map_err(|err| format!("校验音频滤镜失败: {err}"))?;
    if let Some(codec) = encoder.codec() {
        if !codec
            .capabilities()
            .contains(ffmpeg_next::codec::capabilities::Capabilities::VARIABLE_FRAME_SIZE)
        {
            graph
                .get("out")
                .ok_or_else(|| "获取音频输出滤镜失败".to_string())?
                .sink()
                .set_frame_size(encoder.frame_size());
        }
    }
    Ok(graph)
}

fn build_atempo_filter_chain(rate: f64) -> String {
    if (rate - 1.0).abs() <= f64::EPSILON {
        return "atempo=1.0".to_string();
    }
    let mut remaining = rate;
    let mut parts: Vec<String> = Vec::new();
    while remaining > 2.0 + 1e-9 {
        parts.push("atempo=2.0".to_string());
        remaining /= 2.0;
    }
    while remaining < 0.5 - 1e-9 {
        parts.push("atempo=0.5".to_string());
        remaining /= 0.5;
    }
    parts.push(format!("atempo={:.6}", remaining));
    parts.join(",")
}
