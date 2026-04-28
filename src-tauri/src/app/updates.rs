use crate::app::media::playback::debug_log::append_playback_debug_log;
use serde_json::Value;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

fn show_info_dialog(app: &tauri::AppHandle, title: &str, message: &str) {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}

fn show_error_dialog(app: &tauri::AppHandle, title: &str, message: &str) {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}

fn updater_platform_key() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "darwin-aarch64"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "darwin-x86_64"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "windows-aarch64"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows-x86_64"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "linux-aarch64"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux-x86_64"
    } else {
        "unknown"
    }
}

fn configured_updater_endpoints(app: &tauri::AppHandle) -> Vec<String> {
    let Ok(config_value) = serde_json::to_value(app.config()) else {
        return Vec::new();
    };

    config_value
        .get("plugins")
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get("updater"))
        .and_then(Value::as_object)
        .and_then(|updater| updater.get("endpoints"))
        .and_then(Value::as_array)
        .map(|endpoints| {
            endpoints
                .iter()
                .filter_map(|endpoint| endpoint.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn update_log_context(app: &tauri::AppHandle) -> String {
    let endpoints = configured_updater_endpoints(app);
    let endpoint_summary = if endpoints.is_empty() {
        "none".to_string()
    } else {
        endpoints.join(", ")
    };

    format!(
        "platform={}, current_version={}, endpoints={}",
        updater_platform_key(),
        app.package_info().version,
        endpoint_summary
    )
}

fn append_update_log(app: &tauri::AppHandle, stage: &str, message: impl AsRef<str>) {
    let at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default();
    append_playback_debug_log(app, at_ms, stage, message.as_ref());
}

pub async fn check_and_install_update(app: tauri::AppHandle) {
    let context = update_log_context(&app);
    append_update_log(&app, "updater", format!("check started: {context}"));

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(err) => {
            append_update_log(
                &app,
                "updater",
                format!("updater init failed: {err}; {context}"),
            );
            show_error_dialog(
                &app,
                "检查更新失败",
                &format!(
                    "初始化更新器失败：{err}\n平台：{}\n当前版本：{}\n请检查 tauri.conf.json 的 updater 配置。",
                    updater_platform_key(),
                    app.package_info().version
                ),
            );
            return;
        }
    };

    let update = match updater.check().await {
        Ok(update) => update,
        Err(err) => {
            append_update_log(
                &app,
                "updater",
                format!("check failed: {err}; {context}"),
            );
            show_error_dialog(
                &app,
                "检查更新失败",
                &format!(
                    "无法检查更新：{err}\n平台：{}\n当前版本：{}\n请确认 GitHub Releases 与 updater endpoint 已正确配置。",
                    updater_platform_key(),
                    app.package_info().version
                ),
            );
            return;
        }
    };

    let Some(update) = update else {
        append_update_log(&app, "updater", format!("already up to date: {context}"));
        show_info_dialog(&app, "检查更新", "当前已是最新版本。");
        return;
    };

    append_update_log(
        &app,
        "updater",
        format!(
            "update available: target_version={}, body_length={}, {}",
            update.version,
            update.body.as_deref().map(str::len).unwrap_or_default(),
            context
        ),
    );
    show_info_dialog(
        &app,
        "发现新版本",
        &format!("发现新版本 {}，开始自动下载并安装。", update.version),
    );

    let result = update
        .download_and_install(
            |chunk_length, content_length| {
                append_update_log(
                    &app,
                    "updater",
                    format!(
                        "download progress: chunk_length={}, content_length={:?}, {}",
                        chunk_length, content_length, context
                    ),
                );
            },
            || {
                append_update_log(&app, "updater", format!("download finished: {context}"));
            },
        )
        .await;

    if let Err(err) = result {
        append_update_log(
            &app,
            "updater",
            format!("install failed: {err}; {context}"),
        );
        show_error_dialog(&app, "更新失败", &format!("下载或安装更新失败：{err}"));
        return;
    }

    append_update_log(
        &app,
        "updater",
        format!("install finished successfully: new_version={}, {context}", update.version),
    );
    show_info_dialog(&app, "更新完成", "更新已安装，应用将立即重启。");
    app.restart();
}
