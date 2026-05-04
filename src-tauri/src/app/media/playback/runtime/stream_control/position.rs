use crate::app::media::state::MediaState;
use tauri::State;

pub fn write_latest_stream_position(
    state: &State<'_, MediaState>,
    position_seconds: f64,
) -> Result<(), String> {
    state
        .runtime
        .stream
        .set_latest_position_seconds(position_seconds)
        .map_err(|err| err.to_string())
}

pub fn read_latest_stream_position(state: &State<'_, MediaState>) -> Result<f64, String> {
    state
        .runtime
        .stream
        .latest_position_seconds()
        .map_err(|err| err.to_string())
}
