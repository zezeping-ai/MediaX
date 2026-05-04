use super::super::DecodeRuntime;
use crate::app::media::playback::runtime::emit::emit_debug;
use crate::app::media::playback::runtime::session::{
    current_recording_target, update_cache_session_error, CacheRemuxWriter,
};
use ffmpeg_next::Packet;
use tauri::AppHandle;

pub(super) fn update_cache_recording(
    app: &AppHandle,
    source: &str,
    runtime: &mut DecodeRuntime,
    packet: &Packet,
) -> Result<(), String> {
    let recording_target = current_recording_target(app, source)?;
    sync_cache_writer_target(app, source, runtime, recording_target.as_deref());
    if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
        if let Err(err) = writer.write_packet(&runtime.video_ctx.input_ctx, packet) {
            writer.finish();
            runtime.loop_state.cache_writer = None;
            update_cache_session_error(app, source, err.to_string());
            emit_debug(app, "cache_recording_error", err);
        }
    }
    Ok(())
}

fn sync_cache_writer_target(
    app: &AppHandle,
    source: &str,
    runtime: &mut DecodeRuntime,
    recording_target: Option<&str>,
) {
    match (runtime.loop_state.cache_writer.as_ref(), recording_target) {
        (None, Some(target)) => start_cache_writer(app, source, runtime, target),
        (Some(writer), Some(target)) if writer.output_path != target => {
            finish_cache_writer(runtime);
        }
        (Some(_), None) => {
            finish_cache_writer(runtime);
        }
        _ => {}
    }
}

fn start_cache_writer(app: &AppHandle, source: &str, runtime: &mut DecodeRuntime, target: &str) {
    match CacheRemuxWriter::new(&runtime.video_ctx.input_ctx, target) {
        Ok(writer) => {
            emit_debug(
                app,
                "cache_recording",
                format!("start remux recording: {target}"),
            );
            runtime.loop_state.cache_writer = Some(writer);
        }
        Err(err) => {
            update_cache_session_error(app, source, err.clone());
            emit_debug(app, "cache_recording_error", err);
        }
    }
}

fn finish_cache_writer(runtime: &mut DecodeRuntime) {
    if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
        writer.finish();
    }
    runtime.loop_state.cache_writer = None;
}
