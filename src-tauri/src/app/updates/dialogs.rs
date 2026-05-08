use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub(super) fn show_info_dialog(app: &tauri::AppHandle, title: &str, message: &str) {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}

pub(super) fn show_error_dialog(app: &tauri::AppHandle, title: &str, message: &str) {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}
