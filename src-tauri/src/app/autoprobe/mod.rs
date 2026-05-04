use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use std::time::Duration;
use tauri::Manager;

mod actions;
mod env;
mod ready;
mod util;

const AUTOPROBE_SOURCE_ENV: &str = "MEDIAX_AUTOPROBE_SOURCE";
const AUTOPROBE_SOURCE_LIST_ENV: &str = "MEDIAX_AUTOPROBE_SOURCE_LIST";
const AUTOPROBE_SOURCES_DIR_ENV: &str = "MEDIAX_AUTOPROBE_SOURCES_DIR";
const AUTOPROBE_ACTIONS_ENV: &str = "MEDIAX_AUTOPROBE_ACTIONS";
const AUTOPROBE_SOURCE_DWELL_MS_ENV: &str = "MEDIAX_AUTOPROBE_SOURCE_DWELL_MS";
const AUTOPROBE_BETWEEN_SOURCES_MS_ENV: &str = "MEDIAX_AUTOPROBE_BETWEEN_SOURCES_MS";

const AUTOPROBE_READY_TIMEOUT: Duration = Duration::from_secs(8);
const AUTOPROBE_READY_POLL_INTERVAL: Duration = Duration::from_millis(50);
const AUTOPROBE_DEFAULT_SOURCE_DWELL: Duration = Duration::from_secs(45);
const AUTOPROBE_DEFAULT_BETWEEN_SOURCES: Duration = Duration::from_millis(800);

const AUTOPROBE_MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "flv", "webm", "m4v", "ts", "mpeg", "mp3", "aac", "flac", "wav",
];

pub fn bootstrap_from_env(app: &tauri::AppHandle) -> Result<(), String> {
    let sources = env::resolve_sources_from_env(
        app,
        AUTOPROBE_SOURCE_ENV,
        AUTOPROBE_SOURCE_LIST_ENV,
        AUTOPROBE_SOURCES_DIR_ENV,
        AUTOPROBE_MEDIA_EXTENSIONS,
    )?;
    if sources.is_empty() {
        return Ok(());
    }
    let actions = actions::parse_actions_from_env(app, AUTOPROBE_ACTIONS_ENV);
    let source_dwell =
        util::parse_duration_ms_env(AUTOPROBE_SOURCE_DWELL_MS_ENV, AUTOPROBE_DEFAULT_SOURCE_DWELL);
    let between_sources = util::parse_duration_ms_env(
        AUTOPROBE_BETWEEN_SOURCES_MS_ENV,
        AUTOPROBE_DEFAULT_BETWEEN_SOURCES,
    );

    // 延迟 autoplay，避免主窗口/前端还没 ready 导致首帧/seek 预览丢事件。
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        std::thread::sleep(Duration::from_millis(350));
        append_playback_debug_log(
            &app_handle,
            util::now_unix_ms(),
            "autoprobe",
            &format!(
                "bootstrap {} source(s), dwell={}ms, gap={}ms",
                sources.len(),
                source_dwell.as_millis(),
                between_sources.as_millis()
            ),
        );

        for (index, source) in sources.iter().enumerate() {
            append_playback_debug_log(
                &app_handle,
                util::now_unix_ms(),
                "autoprobe_source",
                &format!("start #{}/{}: {}", index + 1, sources.len(), source),
            );

            if let Err(err) = coordinator::open(
                app_handle.clone(),
                app_handle.state::<MediaState>(),
                source.clone(),
                None,
            ) {
                append_playback_debug_log(
                    &app_handle,
                    util::now_unix_ms(),
                    "autoprobe_error",
                    &format!("open failed for `{source}`: {err}"),
                );
                continue;
            }

            if let Err(err) =
                coordinator::play(app_handle.clone(), app_handle.state::<MediaState>(), None)
            {
                append_playback_debug_log(
                    &app_handle,
                    util::now_unix_ms(),
                    "autoprobe_error",
                    &format!("play failed for `{source}`: {err}"),
                );
                continue;
            }

            ready::wait_for_playback_ready(
                &app_handle,
                source,
                "bootstrap",
                AUTOPROBE_READY_TIMEOUT,
                AUTOPROBE_READY_POLL_INTERVAL,
            );
            actions::run_actions(
                &app_handle,
                source,
                &actions,
                AUTOPROBE_READY_TIMEOUT,
                AUTOPROBE_READY_POLL_INTERVAL,
            );
            std::thread::sleep(source_dwell);

            let _ = coordinator::stop(app_handle.clone(), app_handle.state::<MediaState>(), None);
            append_playback_debug_log(
                &app_handle,
                util::now_unix_ms(),
                "autoprobe_source",
                &format!("finish #{}/{}: {}", index + 1, sources.len(), source),
            );

            if index + 1 < sources.len() {
                std::thread::sleep(between_sources);
            }
        }
    });

    Ok(())
}

