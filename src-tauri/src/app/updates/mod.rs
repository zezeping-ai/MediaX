mod config;
mod dialogs;
mod logging;
#[cfg(target_os = "macos")]
mod macos;

use config::{github_release_page_url, update_log_context, updater_platform_key};
use dialogs::{show_error_dialog, show_info_dialog};
use logging::append_update_log;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

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
            append_update_log(&app, "updater", format!("check failed: {err}; {context}"));
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
    if let Some(releases_url) = github_release_page_url(&app) {
        if let Err(err) = app.opener().open_url(&releases_url, None::<String>) {
            append_update_log(
                &app,
                "updater",
                format!("open releases page failed: url={releases_url}, error={err}, {context}"),
            );
        } else {
            append_update_log(
                &app,
                "updater",
                format!("opened releases page: url={releases_url}, {context}"),
            );
        }
    } else {
        append_update_log(
            &app,
            "updater",
            format!("skip opening releases page: no github endpoint found, {context}"),
        );
    }

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
        append_update_log(&app, "updater", format!("install failed: {err}; {context}"));
        show_error_dialog(&app, "更新失败", &format!("下载或安装更新失败：{err}"));
        return;
    }

    append_update_log(
        &app,
        "updater",
        format!(
            "install finished successfully: new_version={}, {context}",
            update.version
        ),
    );
    #[cfg(target_os = "macos")]
    {
        if !macos::ensure_macos_quarantine_ready_for_restart(&app, &context) {
            return;
        }
    }
    show_info_dialog(&app, "更新完成", "更新已安装，应用将立即重启。");
    app.restart();
}
