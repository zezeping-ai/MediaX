use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::playback::session::coordinator;
use crate::app::media::playback::dto::PlaybackStatus;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Manager;

const AUTOPROBE_SOURCE_ENV: &str = "MEDIAX_AUTOPROBE_SOURCE";
const AUTOPROBE_ACTIONS_ENV: &str = "MEDIAX_AUTOPROBE_ACTIONS";
const AUTOPROBE_READY_TIMEOUT: Duration = Duration::from_secs(8);
const AUTOPROBE_READY_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub fn bootstrap_from_env(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(source) = std::env::var(AUTOPROBE_SOURCE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let actions = parse_actions_from_env(app);

    // Delay autoplay until the main window and renderer are ready for a first frame.
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        std::thread::sleep(Duration::from_millis(350));
        let at_ms = now_unix_ms();
        append_playback_debug_log(
            &app_handle,
            at_ms,
            "autoprobe",
            &format!("bootstrap source from env: {source}"),
        );
        if let Err(err) = coordinator::open(
            app_handle.clone(),
            app_handle.state::<MediaState>(),
            source.clone(),
            None,
        ) {
            append_playback_debug_log(
                &app_handle,
                now_unix_ms(),
                "autoprobe_error",
                &format!("open failed: {err}"),
            );
            return;
        }
        if let Err(err) = coordinator::play(app_handle.clone(), app_handle.state::<MediaState>(), None) {
            append_playback_debug_log(
                &app_handle,
                now_unix_ms(),
                "autoprobe_error",
                &format!("play failed: {err}"),
            );
            return;
        }
        wait_for_playback_ready(&app_handle, &source, "bootstrap");
        run_actions(&app_handle, &source, &actions);
    });
    Ok(())
}

#[derive(Debug, Clone)]
enum AutoprobeAction {
    Wait(Duration),
    Pause,
    Resume,
    SetRate(f64),
    SeekAbsolute(f64),
    SeekRelative(f64),
    SetVolume(f64),
    SetMuted(bool),
    SetRouting(PlaybackChannelRouting),
}

fn parse_actions_from_env(app: &tauri::AppHandle) -> Vec<AutoprobeAction> {
    let Some(raw) = std::env::var(AUTOPROBE_ACTIONS_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };

    let mut actions = Vec::new();
    for segment in raw.split(';').map(str::trim).filter(|segment| !segment.is_empty()) {
        match parse_action(segment) {
            Ok(action) => actions.push(action),
            Err(err) => append_playback_debug_log(
                app,
                now_unix_ms(),
                "autoprobe_error",
                &format!("invalid action `{segment}`: {err}"),
            ),
        }
    }
    actions
}

fn parse_action(segment: &str) -> Result<AutoprobeAction, String> {
    let (name, value) = segment
        .split_once(':')
        .ok_or_else(|| "expected `name:value`".to_string())?;
    let name = name.trim().to_ascii_lowercase();
    let value = value.trim();
    match name.as_str() {
        "wait" | "sleep" => Ok(AutoprobeAction::Wait(Duration::from_millis(
            value
                .parse::<u64>()
                .map_err(|err| format!("invalid wait ms: {err}"))?,
        ))),
        "pause" => {
            if !matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "now" | "yes" | "on") {
                return Err("expected true/now to trigger pause".to_string());
            }
            Ok(AutoprobeAction::Pause)
        }
        "resume" | "play" => {
            if !matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "now" | "yes" | "on") {
                return Err("expected true/now to trigger resume".to_string());
            }
            Ok(AutoprobeAction::Resume)
        }
        "rate" => Ok(AutoprobeAction::SetRate(
            value
                .parse::<f64>()
                .map_err(|err| format!("invalid rate: {err}"))?,
        )),
        "seek" => Ok(AutoprobeAction::SeekAbsolute(
            value
                .parse::<f64>()
                .map_err(|err| format!("invalid absolute seek: {err}"))?,
        )),
        "seek_by" => Ok(AutoprobeAction::SeekRelative(
            value
                .parse::<f64>()
                .map_err(|err| format!("invalid relative seek: {err}"))?,
        )),
        "volume" => Ok(AutoprobeAction::SetVolume(
            value
                .parse::<f64>()
                .map_err(|err| format!("invalid volume: {err}"))?,
        )),
        "muted" => Ok(AutoprobeAction::SetMuted(parse_bool(value)?)),
        "routing" => Ok(AutoprobeAction::SetRouting(parse_routing(value)?)),
        _ => Err("unsupported action".to_string()),
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err("expected boolean".to_string()),
    }
}

fn parse_routing(value: &str) -> Result<PlaybackChannelRouting, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "stereo" => Ok(PlaybackChannelRouting::Stereo),
        "left_to_both" => Ok(PlaybackChannelRouting::LeftToBoth),
        "right_to_both" => Ok(PlaybackChannelRouting::RightToBoth),
        _ => Err("expected stereo|left_to_both|right_to_both".to_string()),
    }
}

fn run_actions(app: &tauri::AppHandle, source: &str, actions: &[AutoprobeAction]) {
    for action in actions {
        if let AutoprobeAction::Wait(duration) = action {
            std::thread::sleep(*duration);
            continue;
        }
        let label = describe_action(action);
        append_playback_debug_log(app, now_unix_ms(), "autoprobe_action", &label);
        if let Err(err) = apply_action(app, action) {
            append_playback_debug_log(
                app,
                now_unix_ms(),
                "autoprobe_error",
                &format!("{label} failed: {err}"),
            );
            continue;
        }
        if action_requires_ready_wait(action) {
            wait_for_playback_ready(app, source, &label);
        }
    }
}

fn action_requires_ready_wait(action: &AutoprobeAction) -> bool {
    matches!(
        action,
        AutoprobeAction::Resume
            | AutoprobeAction::SeekAbsolute(_)
            | AutoprobeAction::SeekRelative(_)
    )
}

fn describe_action(action: &AutoprobeAction) -> String {
    match action {
        AutoprobeAction::Wait(duration) => format!("wait {}ms", duration.as_millis()),
        AutoprobeAction::Pause => "pause playback".to_string(),
        AutoprobeAction::Resume => "resume playback".to_string(),
        AutoprobeAction::SetRate(rate) => format!("set rate {rate:.2}x"),
        AutoprobeAction::SeekAbsolute(position) => format!("seek to {position:.3}s"),
        AutoprobeAction::SeekRelative(delta) => format!("seek by {delta:.3}s"),
        AutoprobeAction::SetVolume(volume) => format!("set volume {volume:.2}"),
        AutoprobeAction::SetMuted(muted) => format!("set muted {muted}"),
        AutoprobeAction::SetRouting(routing) => format!("set routing {routing:?}"),
    }
}

fn apply_action(app: &tauri::AppHandle, action: &AutoprobeAction) -> Result<(), String> {
    let state = app.state::<MediaState>();
    match action {
        AutoprobeAction::Wait(_) => Ok(()),
        AutoprobeAction::Pause => coordinator::pause(app.clone(), state, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
        AutoprobeAction::Resume => coordinator::play(app.clone(), state, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
        AutoprobeAction::SetRate(rate) => coordinator::set_rate(app.clone(), state, *rate, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
        AutoprobeAction::SeekAbsolute(position) => {
            coordinator::seek(app.clone(), state, *position, None, None)
                .map(|_| ())
                .map_err(|err| err.to_string())
        }
        AutoprobeAction::SeekRelative(delta) => {
            let current_position = state
                .runtime
                .stream
                .latest_position_seconds()
                .map_err(|err| err.to_string())?;
            let target = (current_position + delta).max(0.0);
            coordinator::seek(app.clone(), state, target, None, None)
                .map(|_| ())
                .map_err(|err| err.to_string())
        }
        AutoprobeAction::SetVolume(volume) => coordinator::set_volume(app.clone(), state, *volume, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
        AutoprobeAction::SetMuted(muted) => coordinator::set_muted(app.clone(), state, *muted, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
        AutoprobeAction::SetRouting(routing) => {
            coordinator::set_channel_routing(app.clone(), state, *routing, None)
                .map(|_| ())
                .map_err(|err| err.to_string())
        }
    }
}

fn wait_for_playback_ready(app: &tauri::AppHandle, source: &str, reason: &str) {
    let start = std::time::Instant::now();
    while start.elapsed() < AUTOPROBE_READY_TIMEOUT {
        if playback_is_ready(app, source) {
            append_playback_debug_log(
                app,
                now_unix_ms(),
                "autoprobe_ready",
                &format!(
                    "{reason} after {}ms",
                    start.elapsed().as_millis(),
                ),
            );
            return;
        }
        std::thread::sleep(AUTOPROBE_READY_POLL_INTERVAL);
    }
    append_playback_debug_log(
        app,
        now_unix_ms(),
        "autoprobe_ready_timeout",
        &format!(
            "{reason} timed out after {}ms",
            AUTOPROBE_READY_TIMEOUT.as_millis(),
        ),
    );
}

fn playback_is_ready(app: &tauri::AppHandle, source: &str) -> bool {
    let state = app.state::<MediaState>();
    let playback_state = match state::playback(&state) {
        Ok(playback) => playback.state(),
        Err(_) => return false,
    };
    if playback_state.current_path.as_deref() != Some(source) {
        return false;
    }
    if playback_state.status != PlaybackStatus::Playing {
        return false;
    }
    if playback_state.duration_seconds > 0.0 && playback_state.position_seconds > 0.0 {
        return true;
    }
    state
        .runtime
        .stream
        .latest_position_seconds()
        .map(|position| position > 0.0)
        .unwrap_or(false)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}
