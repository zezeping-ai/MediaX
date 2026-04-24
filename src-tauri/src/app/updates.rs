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

pub async fn check_and_install_update(app: tauri::AppHandle) {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(err) => {
            show_error_dialog(
                &app,
                "检查更新失败",
                &format!("初始化更新器失败：{err}\n请检查 tauri.conf.json 的 updater 配置。"),
            );
            return;
        }
    };

    let update = match updater.check().await {
        Ok(update) => update,
        Err(err) => {
            show_error_dialog(
                &app,
                "检查更新失败",
                &format!(
                    "无法检查更新：{err}\n请确认 GitHub Releases 与 updater endpoint 已正确配置。"
                ),
            );
            return;
        }
    };

    let Some(update) = update else {
        show_info_dialog(&app, "检查更新", "当前已是最新版本。");
        return;
    };

    show_info_dialog(
        &app,
        "发现新版本",
        &format!("发现新版本 {}，开始自动下载并安装。", update.version),
    );

    let result = update
        .download_and_install(|_chunk_length, _content_length| {}, || {})
        .await;

    if let Err(err) = result {
        show_error_dialog(&app, "更新失败", &format!("下载或安装更新失败：{err}"));
        return;
    }

    show_info_dialog(&app, "更新完成", "更新已安装，应用将立即重启。");
    app.restart();
}
