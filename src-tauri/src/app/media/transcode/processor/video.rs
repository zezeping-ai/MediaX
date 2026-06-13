use super::progress::update_progress;
use super::JobResult;
use crate::app::media::playback::render::video_frame::transfer_hw_frame_if_needed;
use crate::app::media::transcode::state::{TranscodeJob, TranscodeState};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::media::Type;
use ffmpeg_next::picture;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use ffmpeg_next::software::scaling::flag::Flags as ScaleFlags;
use ffmpeg_next::Dictionary;
use ffmpeg_next::Packet;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageBuffer, Rgba};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone)]
struct VideoTranscodePlan {
    container: String,
    video_codec: String,
}

struct VideoTranscoder {
    input_stream_index: usize,
    output_stream_index: usize,
    decoder: ffmpeg_next::decoder::Video,
    encoder: ffmpeg_next::encoder::Video,
    scaler: ScalingContext,
    input_time_base: ffmpeg::Rational,
    output_time_base: ffmpeg::Rational,
}

pub(super) fn transcode_video_with_progress(
    app: &AppHandle,
    job: &TranscodeJob,
    output_path: &Path,
    resolution: &str,
    format: &str,
    rate: f64,
) -> Result<JobResult, String> {
    let plan = parse_video_plan(format)?;
    if plan.container == "gif" {
        return transcode_video_to_gif_with_progress(app, job, output_path, resolution, rate);
    }
    transcode_video_encoded_with_progress(app, job, output_path, resolution, &plan, rate)
}

pub fn probe_video_dimensions(source_path: &str) -> Result<(u32, u32), String> {
    ffmpeg::init().map_err(|err| format!("ffmpeg init failed: {err}"))?;
    let input_ctx = format::input(source_path).map_err(|err| format!("打开输入失败: {err}"))?;
    let Some(video_stream) = input_ctx.streams().best(Type::Video) else {
        return Err("未找到视频流".to_string());
    };
    let decoder = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .map_err(|err| format!("创建视频解码上下文失败: {err}"))?
        .decoder()
        .video()
        .map_err(|err| format!("打开视频解码器失败: {err}"))?;
    Ok((decoder.width(), decoder.height()))
}

fn parse_video_plan(value: &str) -> Result<VideoTranscodePlan, String> {
    let trimmed = value.trim().to_lowercase();
    if trimmed.is_empty() {
        return Ok(VideoTranscodePlan {
            container: "mp4".to_string(),
            video_codec: "h264".to_string(),
        });
    }
    let mut parts = trimmed.splitn(2, '/');
    let container = parts.next().unwrap_or("mp4").to_string();
    let codec_part = parts.next().unwrap_or("h264+aac");
    let video_codec = codec_part.split('+').next().unwrap_or("h264").to_string();
    if container == "gif" {
        return Ok(VideoTranscodePlan {
            container,
            video_codec: "gif".to_string(),
        });
    }
    Ok(VideoTranscodePlan {
        container,
        video_codec,
    })
}

fn parse_resolution_height(resolution: &str) -> Result<Option<u32>, String> {
    let r = resolution.trim().to_lowercase();
    if r.is_empty() || r == "source" {
        return Ok(None);
    }
    let height = match r.as_str() {
        "1080p" => 1080_u32,
        "720p" => 720_u32,
        "480p" => 480_u32,
        "360p" => 360_u32,
        "240p" => 240_u32,
        "120p" => 120_u32,
        _ => return Err(format!("不支持的分辨率: {resolution}")),
    };
    Ok(Some(height))
}

fn compute_target_dimensions(
    source_width: u32,
    source_height: u32,
    target_height: Option<u32>,
) -> (u32, u32) {
    let Some(target_height) = target_height else {
        return (source_width, source_height);
    };
    if target_height >= source_height {
        return (source_width, source_height);
    }
    let ratio = source_width as f64 / source_height as f64;
    let mut width = (target_height as f64 * ratio).round() as u32;
    if !width.is_multiple_of(2) {
        width = width.saturating_sub(1);
    }
    (width.max(2), target_height)
}

fn scale_pts_value(value: Option<i64>, rate: f64) -> Option<i64> {
    value.map(|pts| ((pts as f64) / rate).round() as i64)
}

fn rgb24_frame_to_rgba(frame: &frame::Video) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    let stride = frame.stride(0);
    let data = frame.data(0);
    if width == 0 || height == 0 {
        return Err("GIF 帧尺寸无效".to_string());
    }
    let row_bytes = width * 3;
    if data.len() < stride.saturating_mul(height) {
        return Err("GIF 帧数据不完整".to_string());
    }
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let start = y * stride;
        let end = start + row_bytes;
        let row = &data[start..end];
        for px in row.chunks_exact(3) {
            rgba.push(px[0]);
            rgba.push(px[1]);
            rgba.push(px[2]);
            rgba.push(255);
        }
    }
    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_vec(frame.width(), frame.height(), rgba)
        .ok_or_else(|| "构建 GIF 帧缓冲失败".to_string())
}

fn transcode_video_to_gif_with_progress(
    app: &AppHandle,
    job: &TranscodeJob,
    output_path: &Path,
    resolution: &str,
    rate: f64,
) -> Result<JobResult, String> {
    let mut input_ctx =
        format::input(&job.source_path).map_err(|err| format!("打开输入失败: {err}"))?;
    let duration_ms = {
        let d = input_ctx.duration();
        if d > 0 {
            Some((d as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE) * 1000.0) as u64)
        } else {
            None
        }
    };
    let Some(video_stream) = input_ctx.streams().best(Type::Video) else {
        return Err("未找到视频流".to_string());
    };
    let video_stream_index = video_stream.index();
    let time_base = video_stream.time_base();
    let mut decoder = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .map_err(|err| format!("创建视频解码上下文失败: {err}"))?
        .decoder()
        .video()
        .map_err(|err| format!("打开视频解码器失败: {err}"))?;
    let target_height = parse_resolution_height(resolution)?;
    let (target_width, target_height) =
        compute_target_dimensions(decoder.width(), decoder.height(), target_height);
    let mut scaler = ScalingContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        target_width,
        target_height,
        ScaleFlags::BILINEAR,
    )
    .map_err(|err| format!("创建 GIF 缩放器失败: {err}"))?;
    let gif_file =
        fs::File::create(output_path).map_err(|err| format!("创建 GIF 输出失败: {err}"))?;
    let mut gif_encoder = GifEncoder::new(gif_file);
    gif_encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|err| format!("设置 GIF 循环失败: {err}"))?;
    let gif_fps = 12.0_f64;
    let frame_interval_seconds = 1.0_f64 / gif_fps;
    let mut next_emit_seconds = 0.0_f64;
    let mut emitted_frames = 0_u64;
    let mut decoded = frame::Video::empty();
    let mut last_percent = 0.0_f64;
    for (stream, packet) in input_ctx.packets() {
        let canceled = {
            let state = app.state::<TranscodeState>();
            state.with_inner(|inner| inner.canceled.contains(&job.id))
        };
        if canceled {
            return Ok(JobResult::Skipped(
                fs::metadata(output_path).map(|m| m.len()).unwrap_or(0),
                "任务已取消".to_string(),
            ));
        }
        if stream.index() != video_stream_index {
            continue;
        }
        decoder
            .send_packet(&packet)
            .map_err(|err| format!("向视频解码器送包失败: {err}"))?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            let source_frame = transfer_hw_frame_if_needed(&decoded)
                .map_err(|err| format!("转换视频帧失败: {err}"))?;
            let pts_seconds = source_frame
                .pts()
                .map(|pts| f64::from(time_base) * pts as f64)
                .unwrap_or(next_emit_seconds);
            let adjusted_seconds = pts_seconds / rate;
            if adjusted_seconds + 1e-9 < next_emit_seconds {
                continue;
            }
            let mut rgb_frame = frame::Video::empty();
            scaler
                .run(&source_frame, &mut rgb_frame)
                .map_err(|err| format!("缩放 GIF 帧失败: {err}"))?;
            let rgba = rgb24_frame_to_rgba(&rgb_frame)?;
            let delay_ms = (frame_interval_seconds * 1000.0).round().max(10.0) as u32;
            let gif_frame = Frame::from_parts(rgba, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
            gif_encoder
                .encode_frame(gif_frame)
                .map_err(|err| format!("编码 GIF 帧失败: {err}"))?;
            emitted_frames = emitted_frames.saturating_add(1);
            next_emit_seconds = adjusted_seconds + frame_interval_seconds;
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
        .map_err(|err| format!("发送视频解码 EOF 失败: {err}"))?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        let source_frame = transfer_hw_frame_if_needed(&decoded)
            .map_err(|err| format!("转换视频帧失败: {err}"))?;
        let pts_seconds = source_frame
            .pts()
            .map(|pts| f64::from(time_base) * pts as f64)
            .unwrap_or(next_emit_seconds);
        let adjusted_seconds = pts_seconds / rate;
        if adjusted_seconds + 1e-9 < next_emit_seconds {
            continue;
        }
        let mut rgb_frame = frame::Video::empty();
        scaler
            .run(&source_frame, &mut rgb_frame)
            .map_err(|err| format!("缩放 GIF 帧失败: {err}"))?;
        let rgba = rgb24_frame_to_rgba(&rgb_frame)?;
        let delay_ms = (frame_interval_seconds * 1000.0).round().max(10.0) as u32;
        let gif_frame = Frame::from_parts(rgba, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
        gif_encoder
            .encode_frame(gif_frame)
            .map_err(|err| format!("编码 GIF 帧失败: {err}"))?;
        emitted_frames = emitted_frames.saturating_add(1);
        next_emit_seconds = adjusted_seconds + frame_interval_seconds;
    }
    if emitted_frames == 0 {
        return Err("GIF 转码未产生有效帧".to_string());
    }
    let final_size = fs::metadata(output_path)
        .map(|m| m.len())
        .map_err(|err| format!("读取输出文件元数据失败: {err}"))?;
    Ok(JobResult::Success(final_size))
}

fn transcode_video_encoded_with_progress(
    app: &AppHandle,
    job: &TranscodeJob,
    output_path: &Path,
    resolution: &str,
    plan: &VideoTranscodePlan,
    rate: f64,
) -> Result<JobResult, String> {
    let mut input_ctx =
        format::input(&job.source_path).map_err(|err| format!("打开输入失败: {err}"))?;
    let duration_ms = {
        let d = input_ctx.duration();
        if d > 0 {
            Some((d as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE) * 1000.0) as u64)
        } else {
            None
        }
    };
    let mut output_ctx =
        format::output(output_path).map_err(|err| format!("打开输出失败: {err}"))?;
    let global_header = output_ctx
        .format()
        .flags()
        .contains(format::Flags::GLOBAL_HEADER);
    let Some(video_stream) = input_ctx.streams().best(Type::Video) else {
        return Err("未找到视频流".to_string());
    };
    let video_stream_index = video_stream.index();
    let video_decoder = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .map_err(|err| format!("创建视频解码上下文失败: {err}"))?
        .decoder()
        .video()
        .map_err(|err| format!("打开视频解码器失败: {err}"))?;
    let target_height = parse_resolution_height(resolution)?;
    let (target_width, target_height) =
        compute_target_dimensions(video_decoder.width(), video_decoder.height(), target_height);
    let codec_id = match plan.video_codec.as_str() {
        "h264" | "libx264" => codec::Id::H264,
        "vp9" | "libvpx-vp9" => codec::Id::VP9,
        "mpeg4" => codec::Id::MPEG4,
        "mpeg2" => codec::Id::MPEG2VIDEO,
        "wmv2" => codec::Id::WMV2,
        _ => codec::Id::H264,
    };
    let codec = codec::encoder::find(codec_id);
    let mut out_video_stream = output_ctx
        .add_stream(codec)
        .map_err(|err| format!("创建视频输出流失败: {err}"))?;
    let mut encoder = codec::context::Context::new_with_codec(
        codec.ok_or_else(|| "找不到可用视频编码器".to_string())?,
    )
    .encoder()
    .video()
    .map_err(|err| format!("创建视频编码器失败: {err}"))?;
    out_video_stream.set_parameters(&encoder);
    encoder.set_height(target_height);
    encoder.set_width(target_width);
    encoder.set_aspect_ratio(video_decoder.aspect_ratio());
    encoder.set_format(ffmpeg::format::Pixel::YUV420P);
    encoder.set_time_base(video_stream.time_base());
    encoder.set_frame_rate(video_decoder.frame_rate());
    if global_header {
        encoder.set_flags(codec::Flags::GLOBAL_HEADER);
    }
    let mut opts = Dictionary::new();
    opts.set("preset", "medium");
    opts.set("crf", "24");
    let opened_encoder = encoder
        .open_with(opts)
        .map_err(|err| format!("打开视频编码器失败: {err}"))?;
    out_video_stream.set_parameters(&opened_encoder);
    let scaler = ScalingContext::get(
        video_decoder.format(),
        video_decoder.width(),
        video_decoder.height(),
        ffmpeg::format::Pixel::YUV420P,
        target_width,
        target_height,
        ScaleFlags::BILINEAR,
    )
    .map_err(|err| format!("创建缩放器失败: {err}"))?;
    let mut video_transcoder = VideoTranscoder {
        input_stream_index: video_stream_index,
        output_stream_index: out_video_stream.index(),
        decoder: video_decoder,
        encoder: opened_encoder,
        scaler,
        input_time_base: video_stream.time_base(),
        output_time_base: ffmpeg::Rational(0, 1),
    };
    let mut stream_mapping: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for stream in input_ctx.streams() {
        if stream.index() == video_stream_index {
            continue;
        }
        let medium = stream.parameters().medium();
        if medium != Type::Audio && medium != Type::Subtitle {
            continue;
        }
        let mut out_stream = output_ctx
            .add_stream(codec::encoder::find(codec::Id::None))
            .map_err(|err| format!("创建输出流失败: {err}"))?;
        out_stream.set_parameters(stream.parameters());
        unsafe {
            (*out_stream.parameters().as_mut_ptr()).codec_tag = 0;
        }
        stream_mapping.insert(stream.index(), out_stream.index());
    }
    output_ctx
        .write_header()
        .map_err(|err| format!("写入输出头失败: {err}"))?;
    let out_tb = output_ctx
        .stream(video_transcoder.output_stream_index)
        .ok_or_else(|| "输出视频流不存在".to_string())?
        .time_base();
    video_transcoder.output_time_base = out_tb;
    let mut decoded = frame::Video::empty();
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
        if stream.index() == video_transcoder.input_stream_index {
            video_transcoder
                .decoder
                .send_packet(&packet)
                .map_err(|err| format!("向视频解码器送包失败: {err}"))?;
            while video_transcoder.decoder.receive_frame(&mut decoded).is_ok() {
                let frame_for_scale = transfer_hw_frame_if_needed(&decoded)
                    .map_err(|err| format!("转换视频帧失败: {err}"))?;
                let mut scaled = frame::Video::empty();
                video_transcoder
                    .scaler
                    .run(&frame_for_scale, &mut scaled)
                    .map_err(|err| format!("缩放视频帧失败: {err}"))?;
                scaled.set_pts(scale_pts_value(decoded.pts(), rate));
                scaled.set_kind(picture::Type::None);
                video_transcoder
                    .encoder
                    .send_frame(&scaled)
                    .map_err(|err| format!("向视频编码器送帧失败: {err}"))?;
                while video_transcoder
                    .encoder
                    .receive_packet(&mut encoded)
                    .is_ok()
                {
                    encoded.set_stream(video_transcoder.output_stream_index);
                    encoded.rescale_ts(
                        video_transcoder.input_time_base,
                        video_transcoder.output_time_base,
                    );
                    encoded
                        .write_interleaved(&mut output_ctx)
                        .map_err(|err| format!("写入视频输出包失败: {err}"))?;
                }
            }
        } else if let Some(&out_stream_index) = stream_mapping.get(&stream.index()) {
            let out_stream = output_ctx
                .stream(out_stream_index)
                .ok_or_else(|| "输出流不存在".to_string())?;
            packet.set_pts(scale_pts_value(packet.pts(), rate));
            packet.set_dts(scale_pts_value(packet.dts(), rate));
            packet.rescale_ts(stream.time_base(), out_stream.time_base());
            packet.set_position(-1);
            packet.set_stream(out_stream_index);
            packet
                .write_interleaved(&mut output_ctx)
                .map_err(|err| format!("写入音视频输出包失败: {err}"))?;
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
    video_transcoder
        .decoder
        .send_eof()
        .map_err(|err| format!("发送视频解码 EOF 失败: {err}"))?;
    while video_transcoder.decoder.receive_frame(&mut decoded).is_ok() {
        let frame_for_scale = transfer_hw_frame_if_needed(&decoded)
            .map_err(|err| format!("转换视频帧失败: {err}"))?;
        let mut scaled = frame::Video::empty();
        video_transcoder
            .scaler
            .run(&frame_for_scale, &mut scaled)
            .map_err(|err| format!("缩放视频帧失败: {err}"))?;
        scaled.set_pts(scale_pts_value(decoded.pts(), rate));
        scaled.set_kind(picture::Type::None);
        video_transcoder
            .encoder
            .send_frame(&scaled)
            .map_err(|err| format!("向视频编码器送帧失败: {err}"))?;
        while video_transcoder
            .encoder
            .receive_packet(&mut encoded)
            .is_ok()
        {
            encoded.set_stream(video_transcoder.output_stream_index);
            encoded.rescale_ts(
                video_transcoder.input_time_base,
                video_transcoder.output_time_base,
            );
            encoded
                .write_interleaved(&mut output_ctx)
                .map_err(|err| format!("写入视频输出包失败: {err}"))?;
        }
    }
    video_transcoder
        .encoder
        .send_eof()
        .map_err(|err| format!("发送视频编码 EOF 失败: {err}"))?;
    while video_transcoder
        .encoder
        .receive_packet(&mut encoded)
        .is_ok()
    {
        encoded.set_stream(video_transcoder.output_stream_index);
        encoded.rescale_ts(
            video_transcoder.input_time_base,
            video_transcoder.output_time_base,
        );
        encoded
            .write_interleaved(&mut output_ctx)
            .map_err(|err| format!("写入视频输出包失败: {err}"))?;
    }
    output_ctx
        .write_trailer()
        .map_err(|err| format!("写入输出尾失败: {err}"))?;
    let final_size = fs::metadata(output_path)
        .map(|m| m.len())
        .map_err(|err| format!("读取输出文件元数据失败: {err}"))?;
    Ok(JobResult::Success(final_size))
}
