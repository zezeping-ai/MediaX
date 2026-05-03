use tauri::AppHandle;
#[cfg(not(target_os = "windows"))]
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
#[cfg(target_os = "windows")]
use tauri_plugin_opener::OpenerExt;

pub fn open_default_player_settings(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        if app
            .opener()
            .open_url("ms-settings:defaultapps", None::<String>)
            .is_ok()
        {
            return;
        }
    }

    #[cfg(target_os = "macos")]
    {
        app.dialog()
            .message(
                "请在 Finder 中选中一个音频或视频文件，按 Command+I 打开“显示简介”，在“打开方式”中选择 MediaX，然后点击“全部更改…”。",
            )
            .title("设为默认播放器")
            .kind(MessageDialogKind::Info)
            .show(|_| {});
        return;
    }

    #[cfg(target_os = "linux")]
    {
        app.dialog()
            .message(
                "请在系统默认应用或文件属性中，将音频/视频文件的默认打开方式改为 MediaX。",
            )
            .title("设为默认播放器")
            .kind(MessageDialogKind::Info)
            .show(|_| {});
        return;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        app.dialog()
            .message("当前平台暂未提供默认播放器设置入口。")
            .title("设为默认播放器")
            .kind(MessageDialogKind::Info)
            .show(|_| {});
    }
}
