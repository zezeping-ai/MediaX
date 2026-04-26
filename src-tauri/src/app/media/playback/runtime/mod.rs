use crate::app::media::model::HardwareDecodeMode;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::audio_pipeline::AudioPipeline;
use crate::app::media::playback::runtime::session::DecodeLoopState;
use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;

const MAX_EMIT_FPS: u32 = 60;
const METRICS_EMIT_INTERVAL_MS: u64 = 1000;
const RATE_SWITCH_SETTLE_WINDOW_MS: u64 = 320;
const AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT: f64 = 0.0;
const AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE: f64 = 0.015;
const MAX_DECODE_LEAD_SECONDS_DEFAULT: f64 = 0.25;
const MAX_DECODE_LEAD_SECONDS_DURING_SETTLE: f64 = 0.45;

mod audio;
mod audio_pipeline;
mod clock;
mod emit;
mod loop_driver;
mod progress;
mod runtime_factory;
mod seek_control;
mod session;
mod stream_control;
mod video_pipeline;

use self::emit::{emit_debug, emit_telemetry_payloads};
use self::loop_driver::{finish_decode_runtime, run_decode_loop};
use self::runtime_factory::create_decode_runtime;

pub use stream_control::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream_blocking,
    stop_decode_stream_non_blocking, write_latest_stream_position,
};

struct DecodeRuntime {
    video_ctx: VideoDecodeContext,
    scaler: Option<ScalingContext>,
    audio_pipeline: Option<AudioPipeline>,
    loop_state: DecodeLoopState,
    should_tail_eof: bool,
}

pub(super) fn decode_and_emit_stream(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
    stream_generation: u32,
    hw_mode_override: HardwareDecodeMode,
    software_fallback_reason: Option<String>,
    force_audio_only: bool,
) -> Result<(), String> {
    let mut runtime =
        create_decode_runtime(
            app,
            renderer,
            source,
            audio_controls,
            timing_controls,
            hw_mode_override,
            software_fallback_reason.as_deref(),
            force_audio_only,
        )?;
    run_decode_loop(
        app,
        renderer,
        source,
        stop_flag,
        timing_controls,
        &mut runtime,
        stream_generation,
    )?;
    finish_decode_runtime(
        app,
        renderer,
        stop_flag,
        timing_controls,
        &mut runtime,
        stream_generation,
    )
}
