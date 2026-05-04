use crate::app::media::state::{DecodeStreamHandles, MediaState};
use tauri::State;

pub(super) fn take_decode_stream_handles(
    state: &State<'_, MediaState>,
) -> Result<DecodeStreamHandles, String> {
    state
        .runtime
        .stream
        .take_decode_stream_handles()
        .map_err(|err| err.to_string())
}
