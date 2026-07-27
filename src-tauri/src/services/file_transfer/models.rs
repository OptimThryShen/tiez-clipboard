use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use tauri::AppHandle;
use tokio::sync::broadcast;

pub struct AppState {
    pub app_handle: AppHandle,
    pub ws_tx: broadcast::Sender<String>,
}

#[derive(Default)]
pub struct ServerActivityState {
    pub last_activity: Mutex<Option<SystemTime>>,
}

pub struct WsBroadcaster(pub Mutex<Option<broadcast::Sender<String>>>);

#[derive(Deserialize)]
pub struct ReceiveText {
    pub content: String,
    pub sender_id: String,
    pub sender_name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub direction: String, // "in" = Mobile->PC, "out" = PC->Mobile
    pub msg_type: String,  // "text", "file", "image"
    pub content: String,
    pub timestamp: i64,
    pub sender_id: String,   // Device unique ID
    pub sender_name: String, // Device display name (e.g., "iPhone X", "PC")
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_total: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileBatchMetadata {
    pub id: String,
    pub name: String,
    pub index: usize,
    pub total: usize,
    pub total_size: u64,
}

#[derive(Clone, Serialize)]
pub struct StatusPayload {
    pub enabled: bool,
    pub port: u16,
    pub ip: String,
    pub access_token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub last_seen: i64,
}

pub struct OnlineDevices(pub Mutex<HashMap<String, DeviceInfo>>);

pub struct ChatState(pub Mutex<Vec<Message>>);

impl Default for ChatState {
    fn default() -> Self {
        Self(Mutex::new(Vec::new()))
    }
}

#[derive(Deserialize, Debug)]
pub struct ChunkMetadata {
    pub upload_id: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub file_name: String,
    pub sender_id: String,
    pub sender_name: String,
    pub total_size: u64,
    pub content_type: Option<String>,
    #[serde(default)]
    pub batch_id: Option<String>,
    #[serde(default)]
    pub batch_name: Option<String>,
    #[serde(default)]
    pub batch_index: Option<usize>,
    #[serde(default)]
    pub batch_total: Option<usize>,
    #[serde(default)]
    pub batch_size: Option<u64>,
}

impl ChunkMetadata {
    pub fn batch_metadata(&self) -> Option<FileBatchMetadata> {
        let id = self.batch_id.as_ref()?.trim();
        let total = self.batch_total?;
        let index = self.batch_index?;
        if id.is_empty() || total < 2 || total > 2_000 || index >= total {
            return None;
        }
        let safe_id = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(96)
            .collect::<String>();
        if safe_id.is_empty() {
            return None;
        }
        Some(FileBatchMetadata {
            id: safe_id,
            name: self
                .batch_name
                .as_deref()
                .unwrap_or("文件包")
                .chars()
                .filter(|c| !c.is_control())
                .take(120)
                .collect(),
            index,
            total,
            total_size: self.batch_size.unwrap_or(self.total_size),
        })
    }
}

pub struct UploadSessions(pub Mutex<HashMap<String, std::path::PathBuf>>);

impl Default for UploadSessions {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

#[derive(Clone)]
pub struct SharedFileEntry {
    pub path: String,
    pub expires_at: Instant,
}

impl SharedFileEntry {
    pub fn new(path: String, lifetime: Duration) -> Self {
        Self {
            path,
            expires_at: Instant::now() + lifetime,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

pub struct SharedFileState(pub Mutex<HashMap<String, SharedFileEntry>>);

pub struct ServerInfo {
    pub port: std::sync::atomic::AtomicU16,
    pub ip: Mutex<String>,
    pub access_token: Mutex<String>,
}
