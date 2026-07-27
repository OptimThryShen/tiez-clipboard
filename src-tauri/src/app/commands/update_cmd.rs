use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseBuildInfo {
    pub channel: String,
    pub portable: bool,
}

#[tauri::command]
pub fn get_release_build_info() -> ReleaseBuildInfo {
    let configured_channel = option_env!("TIEZ_RELEASE_CHANNEL").unwrap_or("stable");
    let channel = match configured_channel {
        "beta" => "beta",
        _ => "stable",
    };

    ReleaseBuildInfo {
        channel: channel.to_string(),
        portable: cfg!(feature = "portable"),
    }
}
