use crate::app::media::state::MediaState;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::media::Type;
use ffmpeg_next::Dictionary;
use ffmpeg_next::Packet;
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

pub(crate) struct CacheRemuxWriter {
    pub output_path: String,
    output_ctx: format::context::Output,
    stream_mapping: HashMap<usize, usize>,
}

impl CacheRemuxWriter {
    pub fn new(input_ctx: &format::context::Input, output_path: &str) -> Result<Self, String> {
        let mut output_ctx = format::output(output_path)
            .map_err(|err| format!("open cache output failed: {err}"))?;
        let mut stream_mapping = HashMap::new();
        for stream in input_ctx.streams() {
            let medium = stream.parameters().medium();
            if medium != Type::Video && medium != Type::Audio {
                continue;
            }
            let mut out_stream = output_ctx
                .add_stream(codec::encoder::find(codec::Id::None))
                .map_err(|err| format!("add output stream failed: {err}"))?;
            out_stream.set_parameters(stream.parameters());
            out_stream.set_time_base(stream.time_base());
            stream_mapping.insert(stream.index(), out_stream.index());
        }
        if stream_mapping.is_empty() {
            return Err("cache recording requires at least one audio/video stream".to_string());
        }
        let output_path_lower = output_path.to_ascii_lowercase();
        let mut options = Dictionary::new();
        if output_path_lower.ends_with(".ts") {
            options.set("flush_packets", "1");
        } else {
            options.set("movflags", "frag_keyframe+empty_moov+default_base_moof");
        }
        output_ctx
            .write_header_with(options)
            .map_err(|err| format!("write cache header failed: {err}"))?;
        Ok(Self {
            output_path: output_path.to_string(),
            output_ctx,
            stream_mapping,
        })
    }

    pub fn write_packet(
        &mut self,
        input_ctx: &format::context::Input,
        packet: &Packet,
    ) -> Result<(), String> {
        let Some(&out_stream_index) = self.stream_mapping.get(&packet.stream()) else {
            return Ok(());
        };
        let in_stream = input_ctx
            .stream(packet.stream())
            .ok_or_else(|| "input stream not found".to_string())?;
        let out_stream = self
            .output_ctx
            .stream(out_stream_index)
            .ok_or_else(|| "output stream not found".to_string())?;
        let mut remux_packet = packet.clone();
        remux_packet.set_stream(out_stream_index);
        remux_packet.rescale_ts(in_stream.time_base(), out_stream.time_base());
        remux_packet.set_position(-1);
        remux_packet
            .write_interleaved(&mut self.output_ctx)
            .map_err(|err| format!("write cache packet failed: {err}"))
    }

    pub fn finish(&mut self) {
        let _ = self.output_ctx.write_trailer();
    }
}

pub(crate) fn update_cache_session_error(app: &AppHandle, source: &str, message: String) {
    if let Ok(mut guard) = app.state::<MediaState>().cache_recorder.lock() {
        if let Some(session) = guard.as_mut() {
            if session.source == source {
                session.active = false;
                session.error_message = Some(message);
            }
        }
    }
}
