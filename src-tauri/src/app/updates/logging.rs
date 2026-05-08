use crate::app::media::playback::debug_log::append_playback_debug_log;

pub(super) fn append_update_log(app: &tauri::AppHandle, stage: &str, message: impl AsRef<str>) {
    let at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default();
    append_playback_debug_log(app, at_ms, stage, message.as_ref());
}
