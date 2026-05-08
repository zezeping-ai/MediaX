#[cfg(target_os = "macos")]
use crate::app::updates::dialogs::show_info_dialog;
#[cfg(target_os = "macos")]
use crate::app::updates::logging::append_update_log;

#[cfg(target_os = "macos")]
fn detect_app_bundle_path() -> String {
    let fallback = "/Applications/mediax.app".to_string();
    let Ok(executable) = std::env::current_exe() else {
        return fallback;
    };
    let Some(bundle_path) = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|ext| ext == "app"))
    else {
        return fallback;
    };
    bundle_path.to_string_lossy().to_string()
}

#[cfg(target_os = "macos")]
fn has_macos_quarantine_tag(app_bundle_path: &str) -> bool {
    std::process::Command::new("xattr")
        .arg("-p")
        .arg("com.apple.quarantine")
        .arg(app_bundle_path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn try_clear_macos_quarantine(app_bundle_path: &str) -> Result<(), String> {
    let output = std::process::Command::new("xattr")
        .arg("-d")
        .arg("com.apple.quarantine")
        .arg(app_bundle_path)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!("xattr failed with status {}", output.status))
    } else {
        Err(stderr)
    }
}

#[cfg(target_os = "macos")]
pub(super) fn ensure_macos_quarantine_ready_for_restart(
    app: &tauri::AppHandle,
    context: &str,
) -> bool {
    let app_bundle_path = detect_app_bundle_path();
    if !has_macos_quarantine_tag(&app_bundle_path) {
        append_update_log(
            app,
            "updater",
            format!("quarantine check: no tag found, path={app_bundle_path}, {context}"),
        );
        return true;
    }

    if let Err(err) = try_clear_macos_quarantine(&app_bundle_path) {
        append_update_log(
            app,
            "updater",
            format!("auto clear quarantine failed: path={app_bundle_path}, error={err}, {context}"),
        );
    } else {
        append_update_log(
            app,
            "updater",
            format!("auto clear quarantine succeeded: path={app_bundle_path}, {context}"),
        );
    }

    if !has_macos_quarantine_tag(&app_bundle_path) {
        return true;
    }

    show_info_dialog(
        app,
        "更新完成（需手动授权）",
        &format!(
            "新版已安装，但 macOS 仍拦截启动。\n请先在终端执行：\n\nsudo xattr -d com.apple.quarantine \"{}\"\n\n执行后再手动打开 MediaX。",
            app_bundle_path
        ),
    );
    append_update_log(
        app,
        "updater",
        format!("restart skipped due to quarantine: path={app_bundle_path}, {context}"),
    );
    false
}
