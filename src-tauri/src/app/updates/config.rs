use serde_json::Value;

pub(super) fn updater_platform_key() -> &'static str {
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

pub(super) fn configured_updater_endpoints(app: &tauri::AppHandle) -> Vec<String> {
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

pub(super) fn update_log_context(app: &tauri::AppHandle) -> String {
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

pub(super) fn github_release_page_url(app: &tauri::AppHandle) -> Option<String> {
    configured_updater_endpoints(app).into_iter().find_map(|endpoint| {
        let (repo_base, _) = endpoint.split_once("/releases/")?;
        if repo_base.contains("github.com/") {
            Some(format!("{repo_base}/releases"))
        } else {
            None
        }
    })
}
