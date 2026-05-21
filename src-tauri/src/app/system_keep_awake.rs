use crate::app::media::playback::dto::PlaybackStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

#[cfg(desktop)]
use tauri_plugin_keepawake::{KeepAwakeConfig, TauriPluginKeepawakeExt};

static ACTIVE: AtomicBool = AtomicBool::new(false);

/// 播放中阻止系统/显示器因空闲进入休眠或屏保；暂停/停止后恢复系统默认节能策略。
pub fn sync_for_playback_status(app: &AppHandle, status: &PlaybackStatus) {
    #[cfg(not(desktop))]
    {
        let _ = (app, status);
        return;
    }

    #[cfg(desktop)]
    {
        let should_block = matches!(status, PlaybackStatus::Playing);
        let was_active = ACTIVE.load(Ordering::Relaxed);
        if should_block == was_active {
            return;
        }

        let keepawake = app.tauri_plugin_keepawake();
        let result = if should_block {
            keepawake.start(
                app,
                Some(KeepAwakeConfig {
                    display: true,
                    idle: true,
                    sleep: true,
                }),
            )
        } else {
            keepawake.stop(app)
        };

        match result {
            Ok(()) => ACTIVE.store(should_block, Ordering::Relaxed),
            Err(err) => eprintln!("system keep awake sync failed: {err}"),
        }
    }
}
