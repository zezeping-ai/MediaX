use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::playback::session::coordinator;
use crate::app::media::error::MediaError;
use crate::app::media::state::MediaState;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;

use super::ready::wait_for_playback_ready;
use super::util::{now_unix_ms, parse_bool, parse_routing};

#[derive(Debug, Clone)]
pub(super) enum AutoprobeAction {
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

pub(super) fn parse_actions_from_env(app: &AppHandle, actions_key: &str) -> Vec<AutoprobeAction> {
    let Some(raw) = std::env::var(actions_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };

    let mut actions = Vec::new();
    for segment in raw
        .split(';')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
    {
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

pub(super) fn run_actions(
    app: &AppHandle,
    source: &str,
    actions: &[AutoprobeAction],
    ready_timeout: Duration,
    ready_poll_interval: Duration,
) {
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
            wait_for_playback_ready(
                app,
                source,
                &label,
                ready_timeout,
                ready_poll_interval,
            );
        }
    }
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
            if !matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "now" | "yes" | "on"
            ) {
                return Err("expected true/now to trigger pause".to_string());
            }
            Ok(AutoprobeAction::Pause)
        }
        "resume" | "play" => {
            if !matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "now" | "yes" | "on"
            ) {
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

fn action_requires_ready_wait(action: &AutoprobeAction) -> bool {
    matches!(
        action,
        AutoprobeAction::Resume | AutoprobeAction::SeekAbsolute(_) | AutoprobeAction::SeekRelative(_)
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

fn apply_action(app: &AppHandle, action: &AutoprobeAction) -> Result<(), String> {
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
        AutoprobeAction::SeekAbsolute(position) => coordinator::seek(app.clone(), state, *position, None, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
        AutoprobeAction::SeekRelative(delta) => {
            let current_position = state
                .runtime
                .stream
                .latest_position_seconds()
                .map_err(|err: MediaError| err.to_string())?;
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
        AutoprobeAction::SetRouting(routing) => coordinator::set_channel_routing(app.clone(), state, *routing, None)
            .map(|_| ())
            .map_err(|err| err.to_string()),
    }
}

