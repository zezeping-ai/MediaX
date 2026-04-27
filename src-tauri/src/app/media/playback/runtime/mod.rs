mod audio;
mod audio_pipeline;
mod clock;
mod constants;
mod decode_params;
mod decode_runtime;
mod emit;
mod loop_driver;
mod progress;
mod runtime_factory;
mod seek_control;
mod session;
mod stream_control;
mod video_pipeline;

pub(crate) use self::constants::*;
pub(crate) use self::decode_params::{DecodeDependencies, DecodeRequest};
use self::decode_runtime::DecodeRuntime;
use self::emit::{emit_debug, emit_telemetry_payloads};
use self::loop_driver::{finish_decode_runtime, run_decode_loop};
use self::runtime_factory::create_decode_runtime;

pub use stream_control::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream_blocking,
    stop_decode_stream_non_blocking, write_latest_stream_position,
};

pub(super) fn decode_and_emit_stream(
    dependencies: DecodeDependencies<'_>,
    request: DecodeRequest<'_>,
) -> Result<(), String> {
    let mut runtime = create_decode_runtime(&dependencies, &request)?;
    run_decode_loop(
        dependencies.app,
        dependencies.renderer,
        request.source,
        dependencies.stop_flag,
        dependencies.timing_controls,
        &mut runtime,
        request.stream_generation,
    )?;
    finish_decode_runtime(
        dependencies.app,
        dependencies.renderer,
        dependencies.stop_flag,
        dependencies.timing_controls,
        &mut runtime,
        request.stream_generation,
    )
}
