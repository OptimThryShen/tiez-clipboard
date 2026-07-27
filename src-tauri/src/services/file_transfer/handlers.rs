use axum::{
    body::Body,
    extract::{
        ws::{Message as WsMessage, WebSocket},
        Multipart, Path, Query, Request, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Json, Response},
};
use base64::Engine;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::app_state::{SessionHistory, SettingsState};
use crate::database::ClipboardEntry;
use crate::database::DbState;
use crate::infrastructure::repository::clipboard_repo::ClipboardRepository;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

use super::models::*;
use super::utils::*;
use super::web_ui::render_index;
use super::{
    append_message, append_message_with_batch, register_received_file, register_shared_file,
};

const TRANSFER_CHUNK_SIZE: usize = 512 * 1024;
const MIN_DISK_RESERVE: u64 = 512 * 1024 * 1024;
const MAX_DISK_RESERVE: u64 = 5 * 1024 * 1024 * 1024;

fn ensure_disk_capacity(path: &std::path::Path, required: u64) -> Result<(), &'static str> {
    let available = fs2::available_space(path).map_err(|_| "Unable to inspect free disk space")?;
    let total = fs2::total_space(path).map_err(|_| "Unable to inspect disk capacity")?;
    let reserve = (total / 50).clamp(MIN_DISK_RESERVE, MAX_DISK_RESERVE);
    if available.saturating_sub(reserve) < required {
        Err("Not enough disk space while preserving the system safety reserve")
    } else {
        Ok(())
    }
}

fn safe_file_name(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let base = normalized.rsplit('/').next().unwrap_or("");
    let cleaned: String = base
        .chars()
        .filter(|ch| !ch.is_control() && *ch != '/' && *ch != '\\')
        .take(240)
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        "unnamed-file".to_string()
    } else {
        cleaned.to_string()
    }
}

fn safe_upload_id(value: &str) -> Option<&str> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Some(value)
    } else {
        None
    }
}

fn safe_identity(value: &str, fallback: &str) -> String {
    let value: String = value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(128)
        .collect();
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

const UPLOAD_SESSION_TTL: std::time::Duration = std::time::Duration::from_secs(60 * 60);
const MAX_UPLOAD_SESSIONS: usize = 256;

fn prune_upload_sessions(
    sessions: &mut std::collections::HashMap<String, std::path::PathBuf>,
) {
    let stale_keys = sessions
        .iter()
        .filter_map(|(upload_id, path)| {
            let is_stale = std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .map(|elapsed| elapsed >= UPLOAD_SESSION_TTL)
                .unwrap_or(false);
            is_stale.then(|| upload_id.clone())
        })
        .collect::<Vec<_>>();

    for upload_id in stale_keys {
        if let Some(path) = sessions.remove(&upload_id) {
            let _ = std::fs::remove_file(path);
        }
    }

    while sessions.len() >= MAX_UPLOAD_SESSIONS {
        let Some(upload_id) = sessions.keys().next().cloned() else {
            break;
        };
        if let Some(path) = sessions.remove(&upload_id) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn validate_chunk(meta: &ChunkMetadata, data_len: usize) -> Result<(), &'static str> {
    if safe_upload_id(&meta.upload_id).is_none() {
        return Err("Invalid upload id");
    }
    let expected_chunks = std::cmp::max(
        1_u64,
        meta.total_size
            .saturating_add(TRANSFER_CHUNK_SIZE as u64 - 1)
            / TRANSFER_CHUNK_SIZE as u64,
    );
    if meta.total_chunks as u64 != expected_chunks || meta.chunk_index >= meta.total_chunks {
        return Err("Invalid chunk metadata");
    }
    let expected_len = if meta.total_size == 0 {
        0_u64
    } else if meta.chunk_index + 1 == meta.total_chunks {
        meta.total_size
            .saturating_sub(meta.chunk_index as u64 * TRANSFER_CHUNK_SIZE as u64)
    } else {
        TRANSFER_CHUNK_SIZE as u64
    };
    if data_len as u64 != expected_len {
        return Err("Invalid chunk size");
    }
    Ok(())
}

fn parse_single_byte_range(value: &str, total_size: u64) -> Option<(u64, u64)> {
    if total_size == 0 {
        return None;
    }

    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;

    if start.is_empty() {
        let suffix_length = end.parse::<u64>().ok()?;
        if suffix_length == 0 {
            return None;
        }
        let length = suffix_length.min(total_size);
        return Some((total_size - length, total_size - 1));
    }

    let start = start.parse::<u64>().ok()?;
    if start >= total_size {
        return None;
    }
    let end = if end.is_empty() {
        total_size - 1
    } else {
        end.parse::<u64>().ok()?.min(total_size - 1)
    };
    (end >= start).then_some((start, end))
}

fn request_token(request: &Request) -> Option<String> {
    if let Some(value) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    {
        return Some(value.to_string());
    }

    if let Some(value) = request
        .uri()
        .query()
        .and_then(|query| query.split('&').find_map(|part| part.strip_prefix("auth=")))
        .and_then(|value| urlencoding::decode(value).ok())
    {
        return Some(value.into_owned());
    }

    request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == "tiez_ft_session").then(|| value.to_string())
            })
        })
}

fn apply_security_headers(response: &mut Response) {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    );
}

pub async fn apply_download_security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    apply_security_headers(&mut response);
    response
}

pub async fn require_session(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let expected = state
        .app_handle
        .state::<ServerInfo>()
        .access_token
        .lock()
        .map(|token| token.clone())
        .unwrap_or_default();
    let supplied = request_token(&request);
    let from_query = request
        .uri()
        .query()
        .is_some_and(|query| query.split('&').any(|part| part.starts_with("auth=")));

    let is_preflight = request.method() == Method::OPTIONS;
    let mut response = if is_preflight
        || (!expected.is_empty() && supplied.as_deref() == Some(expected.as_str()))
    {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired transfer session",
        )
            .into_response()
    };

    if from_query && response.status() != StatusCode::UNAUTHORIZED {
        if let Ok(cookie) = HeaderValue::from_str(&format!(
            "tiez_ft_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=28800",
            expected
        )) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
    }
    apply_security_headers(&mut response);
    response
}

fn with_cors(mut response: axum::response::Response) -> axum::response::Response {
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("*"),
    );
    response
}

pub async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let app_handle = &state.app_handle;
    let settings = app_handle.state::<SettingsState>();
    let db_state = app_handle.state::<DbState>();
    let logo_base64 = get_app_logo_base64(app_handle);
    let theme = {
        let guard = settings.theme.lock().unwrap();
        guard.clone()
    };
    let color_mode = db_state
        .settings_repo
        .get("app.color_mode")
        .ok()
        .flatten()
        .unwrap_or_else(|| "system".to_string());

    Html(render_index(&theme, &color_mode, &logo_base64))
}

pub async fn poll_messages(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<Message>> {
    let last_id = params
        .get("last_id")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let chat_state = state.app_handle.state::<ChatState>();

    let msgs_result = {
        match chat_state.0.lock() {
            Ok(msgs) => msgs
                .iter()
                .filter(|m| m.id > last_id)
                .map(|m| {
                    let mut m_clone = m.clone();
                    if matches!(m.msg_type.as_str(), "image" | "video" | "file")
                        && !m.content.starts_with("data:")
                    {
                        if let Some(shared_path) = m.file_path.as_deref() {
                            let path = std::path::Path::new(shared_path);
                            let filename = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();

                            if let Some(token) =
                                register_shared_file(&state.app_handle, shared_path.to_string())
                            {
                                m_clone.content = format!(
                                    "/download/{}?name={}",
                                    token,
                                    urlencoding::encode(&filename)
                                );
                            }
                        } else if !m.content.starts_with("/download/") {
                            let path = std::path::Path::new(&m.content);
                            let filename = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();
                            if let Some(token) =
                                register_shared_file(&state.app_handle, m.content.clone())
                            {
                                m_clone.content = format!(
                                    "/download/{}?name={}",
                                    token,
                                    urlencoding::encode(&filename)
                                );
                            }
                        }
                    }
                    m_clone.file_path = None;
                    m_clone
                })
                .collect::<Vec<Message>>(),
            Err(_) => vec![],
        }
    };

    Json(msgs_result)
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum WsIncoming {
    #[serde(rename = "identity")]
    Identity {
        device_id: String,
        device_name: String,
    },
}

pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();
    let mut current_device_id: Option<String> = None;

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let state_inner = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                    match incoming {
                        WsIncoming::Identity {
                            device_id,
                            device_name,
                        } => {
                            let device_id = safe_identity(&device_id, "mobile");
                            let device_name = safe_identity(&device_name, "手机");
                            let online_devices = state_inner.app_handle.state::<OnlineDevices>();
                            {
                                let mut guard = online_devices.0.lock().unwrap();
                                guard.insert(
                                    device_id.clone(),
                                    DeviceInfo {
                                        id: device_id.clone(),
                                        name: device_name,
                                        last_seen: chrono::Utc::now().timestamp_millis(),
                                    },
                                );
                                current_device_id = Some(device_id);

                                let devices: Vec<DeviceInfo> = guard.values().cloned().collect();
                                let update = serde_json::json!({
                                    "type": "devices_update",
                                    "devices": devices
                                });
                                let _ = state_inner.ws_tx.send(update.to_string());
                                let _ = state_inner
                                    .app_handle
                                    .emit("online-devices-updated", devices);
                            }
                        }
                    }
                }
            }
        }

        if let Some(id) = current_device_id {
            let online_devices = state_inner.app_handle.state::<OnlineDevices>();
            {
                let mut guard = online_devices.0.lock().unwrap();
                guard.remove(&id);
                let devices: Vec<DeviceInfo> = guard.values().cloned().collect();
                let update = serde_json::json!({
                    "type": "devices_update",
                    "devices": devices
                });
                let _ = state_inner.ws_tx.send(update.to_string());
                let _ = state_inner
                    .app_handle
                    .emit("online-devices-updated", devices);
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

pub async fn handle_text(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReceiveText>,
) -> axum::response::Response {
    let db_state = state.app_handle.state::<DbState>();
    update_activity(&state.app_handle);

    if payload.content.len() > 1024 * 1024 {
        return (StatusCode::PAYLOAD_TOO_LARGE, "Text is too large").into_response();
    }
    let sender_id = safe_identity(&payload.sender_id, "mobile");
    let sender_name = safe_identity(&payload.sender_name, "手机");

    append_message(
        &state.app_handle,
        "in",
        "text",
        &payload.content,
        &sender_id,
        &sender_name,
        None,
    );

    let settings = state.app_handle.state::<SettingsState>();
    let session_hist = state.app_handle.state::<SessionHistory>();

    let mut preview = payload.content.clone();
    if preview.chars().count() > 100 {
        preview = preview.chars().take(100).collect();
        preview.push_str("...");
    }

    let mut id_result = Ok(0);

    if settings.auto_copy_file.load(Ordering::Relaxed) {
        id_result = if settings.persistent.load(Ordering::Relaxed) {
            let entry = ClipboardEntry {
                id: 0,
                content_type: "text".to_string(),
                content: payload.content.clone(),
                html_content: None,
                source_app: sender_name.to_string(),
                source_app_path: None,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
                preview: preview.clone(),
                is_pinned: false,
                tags: Vec::new(),
                note: String::new(),
                use_count: 0,
                is_external: false,
                pinned_order: 0,
                file_preview_exists: true,
            };
            db_state.repo.save(&entry, None).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })
        } else {
            let id = -(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64
                / 1000);
            let entry = ClipboardEntry {
                id,
                content_type: "text".to_string(),
                content: payload.content.clone(),
                html_content: None,
                source_app: "File Transfer".to_string(),
                source_app_path: None,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
                preview: preview.clone(),
                is_pinned: false,
                tags: Vec::new(),
                note: String::new(),
                use_count: 0,
                is_external: false,
                pinned_order: 0,
                file_preview_exists: true,
            };

            if let Ok(mut session) = session_hist.0.lock() {
                session.push_back(entry);
                if session.len() > 500 {
                    if let Some(removed) = session.pop_front() {
                        let _ = state.app_handle.emit("clipboard-removed", removed.id);
                    }
                }
            }
            Ok(id)
        };
    }

    if let Ok(id) = id_result {
        if id != 0 {
            let _ = state.app_handle.emit("clipboard-changed", id);
            return (StatusCode::OK, "Text received").into_response();
        }
    }
    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save text").into_response()
}

pub async fn upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    let mut success = false;
    let db_state = state.app_handle.state::<DbState>();

    let mut current_sender_id = "mobile".to_string();
    let mut current_sender_name = "手机".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "sender_id" {
            if let Ok(val) = field.text().await {
                current_sender_id = safe_identity(&val, "mobile");
            }
            continue;
        }
        if name == "sender_name" {
            if let Ok(val) = field.text().await {
                current_sender_name = safe_identity(&val, "手机");
            }
            continue;
        }

        if name == "file" {
            let file_name = safe_file_name(field.file_name().unwrap_or("unknown.txt"));
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let mut save_dir = state
                .app_handle
                .path()
                .download_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            if let Ok(Some(custom)) = db_state.settings_repo.get("file_transfer_path") {
                if !custom.trim().is_empty() {
                    save_dir = std::path::PathBuf::from(custom);
                }
            }
            if !save_dir.exists() {
                let _ = std::fs::create_dir_all(&save_dir);
            }

            let target_path = save_dir.join(format!(
                "{}_{}",
                chrono::Utc::now().format("%Y%m%d%H%M%S"),
                file_name
            ));

            if let Ok(mut file) = File::create(&target_path).await {
                let mut stream = field;
                let mut write_success = true;
                while let Some(Ok(chunk)) = stream.next().await {
                    if ensure_disk_capacity(&save_dir, chunk.len() as u64).is_err() {
                        write_success = false;
                        break;
                    }
                    if let Err(e) = file.write_all(&chunk).await {
                        eprintln!("Error writing: {}", e);
                        write_success = false;
                        break;
                    }
                }

                if write_success {
                    register_received_file(
                        &state.app_handle,
                        target_path,
                        file_name,
                        content_type,
                        current_sender_id.clone(),
                        current_sender_name.clone(),
                        None,
                        None,
                    )
                    .await;
                    success = true;
                } else {
                    let _ = tokio::fs::remove_file(&target_path).await;
                }
            }
        }
    }

    if success {
        (StatusCode::OK, "Upload successful").into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Upload failed").into_response()
    }
}

pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let mut metadata: Option<ChunkMetadata> = None;
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "metadata" {
            if let Ok(bytes) = field.bytes().await {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(m) = serde_json::from_str(&text) {
                        metadata = Some(m);
                    }
                }
            }
        } else if name == "data" || name == "file" {
            if let Ok(bytes) = field.bytes().await {
                chunk_data = Some(bytes.to_vec());
            }
        }
    }

    let meta = match metadata {
        Some(m) => m,
        None => return (StatusCode::BAD_REQUEST, "Missing metadata").into_response(),
    };
    let data = match chunk_data {
        Some(d) => d,
        None => return (StatusCode::BAD_REQUEST, "Missing data").into_response(),
    };

    process_upload_chunk(state, meta, data).await
}

#[derive(serde::Deserialize)]
pub struct Base64ChunkPayload {
    metadata: ChunkMetadata,
    data_base64: String,
}

pub async fn upload_chunk_base64(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Base64ChunkPayload>,
) -> axum::response::Response {
    let data = match base64::engine::general_purpose::STANDARD.decode(payload.data_base64.as_bytes())
    {
        Ok(data) => data,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid base64 chunk").into_response(),
    };
    process_upload_chunk(state, payload.metadata, data).await
}

async fn process_upload_chunk(
    state: Arc<AppState>,
    meta: ChunkMetadata,
    data: Vec<u8>,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    if let Err(message) = validate_chunk(&meta, data.len()) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }
    let safe_name = safe_file_name(&meta.file_name);

    let sessions = state.app_handle.state::<UploadSessions>();
    let temp_path = {
        let mut sessions_map = sessions.0.lock().unwrap();
        prune_upload_sessions(&mut sessions_map);
        sessions_map
            .entry(meta.upload_id.clone())
            .or_insert_with(|| {
                let mut path = state
                    .app_handle
                    .path()
                    .download_dir()
                    .unwrap_or_else(|_| std::env::temp_dir());
                if let Ok(Some(custom)) = state
                    .app_handle
                    .state::<DbState>()
                    .settings_repo
                    .get("file_transfer_path")
                {
                    if !custom.trim().is_empty() {
                        path = std::path::PathBuf::from(custom);
                    }
                }
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                path.join(format!(".tmp_{}", meta.upload_id))
            })
            .clone()
    };

    let mut options = tokio::fs::OpenOptions::new();
    let required_space = if meta.chunk_index == 0 {
        meta.total_size
    } else {
        data.len() as u64
    };
    if ensure_disk_capacity(temp_path.parent().unwrap_or(&temp_path), required_space).is_err() {
        return (
            StatusCode::INSUFFICIENT_STORAGE,
            "Not enough available disk space",
        )
            .into_response();
    }
    options.create(true).write(true);
    if meta.chunk_index == 0 {
        options.truncate(true);
    } else {
        let expected_offset = meta.chunk_index as u64 * TRANSFER_CHUNK_SIZE as u64;
        let current_size = tokio::fs::metadata(&temp_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if current_size != expected_offset {
            return (StatusCode::CONFLICT, "Chunk received out of order").into_response();
        }
        options.append(true);
    }

    if let Ok(mut file) = options.open(&temp_path).await {
        if let Err(e) = file.write_all(&data).await {
            eprintln!("Error writing chunk: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Write failed").into_response();
        }
    } else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Open failed").into_response();
    }

    if meta.chunk_index == meta.total_chunks - 1 {
        let batch = meta.batch_metadata();
        let file_size = Some(meta.total_size);
        let final_filename = format!(
            "{}_{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S"),
            safe_name
        );
        let final_path = temp_path.parent().unwrap().join(&final_filename);

        if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
            eprintln!("Error finalizing file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response();
        }

        {
            let mut sessions_map = sessions.0.lock().unwrap();
            sessions_map.remove(&meta.upload_id);
        }

        let content_type = meta
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
        register_received_file(
            &state.app_handle,
            final_path,
            safe_name,
            content_type,
            meta.sender_id,
            meta.sender_name,
            batch,
            file_size,
        )
        .await;

        return (StatusCode::OK, "Upload complete").into_response();
    }

    (StatusCode::OK, "Chunk received").into_response()
}

pub async fn share_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    let mut metadata: Option<ChunkMetadata> = None;
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "metadata" {
            if let Ok(bytes) = field.bytes().await {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(m) = serde_json::from_str(&text) {
                        metadata = Some(m);
                    }
                }
            }
        } else if name == "data" || name == "file" {
            if let Ok(bytes) = field.bytes().await {
                chunk_data = Some(bytes.to_vec());
            }
        }
    }

    let meta = match metadata {
        Some(m) => m,
        None => return with_cors((StatusCode::BAD_REQUEST, "Missing metadata").into_response()),
    };
    let data = match chunk_data {
        Some(d) => d,
        None => return with_cors((StatusCode::BAD_REQUEST, "Missing data").into_response()),
    };
    if let Err(message) = validate_chunk(&meta, data.len()) {
        return with_cors((StatusCode::BAD_REQUEST, message).into_response());
    }
    let safe_name = safe_file_name(&meta.file_name);

    let sessions = state.app_handle.state::<UploadSessions>();
    let temp_path = {
        let mut sessions_map = sessions.0.lock().unwrap();
        prune_upload_sessions(&mut sessions_map);
        sessions_map
            .entry(meta.upload_id.clone())
            .or_insert_with(|| {
                let path = std::env::temp_dir().join("tiez_shared_uploads");
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                path.join(format!(".tmp_share_{}", meta.upload_id))
            })
            .clone()
    };

    let mut options = tokio::fs::OpenOptions::new();
    let required_space = if meta.chunk_index == 0 {
        meta.total_size
    } else {
        data.len() as u64
    };
    if ensure_disk_capacity(temp_path.parent().unwrap_or(&temp_path), required_space).is_err() {
        return with_cors(
            (
                StatusCode::INSUFFICIENT_STORAGE,
                "Not enough available disk space",
            )
                .into_response(),
        );
    }
    options.create(true).write(true);
    if meta.chunk_index == 0 {
        options.truncate(true);
    } else {
        let expected_offset = meta.chunk_index as u64 * TRANSFER_CHUNK_SIZE as u64;
        let current_size = tokio::fs::metadata(&temp_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if current_size != expected_offset {
            return with_cors(
                (StatusCode::CONFLICT, "Chunk received out of order").into_response(),
            );
        }
        options.append(true);
    }

    if let Ok(mut file) = options.open(&temp_path).await {
        if let Err(e) = file.write_all(&data).await {
            eprintln!("Error writing shared chunk: {}", e);
            return with_cors((StatusCode::INTERNAL_SERVER_ERROR, "Write failed").into_response());
        }
    } else {
        return with_cors((StatusCode::INTERNAL_SERVER_ERROR, "Open failed").into_response());
    }

    if meta.chunk_index == meta.total_chunks.saturating_sub(1) {
        let batch = meta.batch_metadata();
        let file_size = Some(meta.total_size);
        let final_filename = format!(
            "share_{}_{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S"),
            safe_name
        );
        let final_path = temp_path.parent().unwrap().join(&final_filename);

        if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
            eprintln!("Error finalizing shared file: {}", e);
            return with_cors(
                (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response(),
            );
        }

        {
            let mut sessions_map = sessions.0.lock().unwrap();
            sessions_map.remove(&meta.upload_id);
        }

        let content_type = meta
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let is_image = content_type.starts_with("image/");
        let is_video = content_type.starts_with("video/");
        let file_name_lower = safe_name.to_lowercase();
        let msg_type = if is_image {
            "image"
        } else if is_video
            || [".mp4", ".mov", ".mkv", ".avi", ".wmv", ".flv", ".webm"]
                .iter()
                .any(|ext| file_name_lower.ends_with(ext))
        {
            "video"
        } else {
            "file"
        };

        let shared_path = final_path.to_string_lossy().to_string();
        append_message_with_batch(
            &state.app_handle,
            "out",
            msg_type,
            &shared_path,
            "pc",
            "电脑",
            Some(&shared_path),
            batch.as_ref(),
            file_size,
        );

        return with_cors((StatusCode::OK, "Share upload complete").into_response());
    }

    with_cors((StatusCode::OK, "Shared chunk received").into_response())
}

pub async fn share_chunk_options() -> axum::response::Response {
    with_cors(StatusCode::NO_CONTENT.into_response())
}

#[derive(serde::Serialize)]
pub struct BatchDownloadResponse {
    url: String,
    name: String,
}

pub async fn prepare_batch_download(
    Path(batch_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    if batch_id.len() > 96
        || !batch_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return (StatusCode::BAD_REQUEST, "Invalid package id").into_response();
    }

    let (batch_name, files, expected_total) = {
        let chat_state = state.app_handle.state::<ChatState>();
        let messages = match chat_state.0.lock() {
            Ok(messages) => messages,
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "History unavailable")
                    .into_response()
            }
        };
        let matching = messages
            .iter()
            .filter(|message| message.batch_id.as_deref() == Some(batch_id.as_str()))
            .collect::<Vec<_>>();
        let expected_total = matching
            .iter()
            .filter_map(|message| message.batch_total)
            .max()
            .unwrap_or(0);
        let name = matching
            .iter()
            .find_map(|message| message.batch_name.clone())
            .unwrap_or_else(|| "TieZ 文件包".to_string());
        let files = matching
            .iter()
            .filter_map(|message| message.file_path.as_deref())
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        (name, files, expected_total)
    };

    if expected_total < 2 || files.len() != expected_total {
        return (
            StatusCode::CONFLICT,
            "Package is incomplete or its files are no longer available",
        )
            .into_response();
    }

    let total_size = files.iter().fold(0_u64, |total, path| {
        total.saturating_add(
            std::fs::metadata(path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
        )
    });
    let mut archive_dir = state
        .app_handle
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    archive_dir.push("file-transfer-packages");
    if std::fs::create_dir_all(&archive_dir).is_err()
        || ensure_disk_capacity(&archive_dir, total_size).is_err()
    {
        return (
            StatusCode::INSUFFICIENT_STORAGE,
            "Not enough space to prepare this package",
        )
            .into_response();
    }

    let safe_package_name = safe_file_name(&batch_name);
    let download_name = if safe_package_name.to_lowercase().ends_with(".zip") {
        safe_package_name
    } else {
        format!("{safe_package_name}.zip")
    };
    let archive_path = archive_dir.join(format!(
        ".tiez-batch-{batch_id}-{}.zip",
        uuid::Uuid::new_v4()
    ));
    let archive_path_for_job = archive_path.clone();
    let archive_result = tokio::task::spawn_blocking(move || {
        create_batch_archive(&archive_path_for_job, &files)
    })
    .await;

    if !matches!(archive_result, Ok(Ok(()))) {
        let _ = std::fs::remove_file(&archive_path);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Unable to create package").into_response();
    }

    let Some(token) = register_shared_file(
        &state.app_handle,
        archive_path.to_string_lossy().to_string(),
    ) else {
        let _ = std::fs::remove_file(&archive_path);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Unable to share package").into_response();
    };

    let cleanup_path = archive_path.clone();
    tokio::spawn(async move {
        // Match the download token lifetime so large packages are not removed
        // while a slow LAN client is still preparing or downloading them.
        tokio::time::sleep(std::time::Duration::from_secs(8 * 60 * 60)).await;
        let _ = tokio::fs::remove_file(cleanup_path).await;
    });

    Json(BatchDownloadResponse {
        url: format!(
            "/download/{}?name={}",
            token,
            urlencoding::encode(&download_name)
        ),
        name: download_name,
    })
    .into_response()
}

fn create_batch_archive(
    archive_path: &std::path::Path,
    files: &[std::path::PathBuf],
) -> Result<(), String> {
    use std::collections::HashSet;
    use std::io;
    use zip::write::SimpleFileOptions;

    let archive_file = std::fs::File::create(archive_path).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(archive_file);
    let mut used_names = HashSet::new();

    for path in files {
        let original_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        let safe_name = unique_package_entry_name(&safe_file_name(original_name), &mut used_names);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let already_compressed = [
            "zip", "rar", "7z", "gz", "bz2", "xz", "jpg", "jpeg", "png", "gif", "webp",
            "heic", "avif", "mp3", "aac", "m4a", "mp4", "mov", "mkv", "webm", "pdf",
        ]
        .contains(&extension.as_str());
        let method = if already_compressed {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        let options = SimpleFileOptions::default().compression_method(method);
        writer
            .start_file(safe_name, options)
            .map_err(|e| e.to_string())?;
        let mut source = std::fs::File::open(path).map_err(|e| e.to_string())?;
        io::copy(&mut source, &mut writer).map_err(|e| e.to_string())?;
    }

    writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn unique_package_entry_name(
    requested: &str,
    used_names: &mut std::collections::HashSet<String>,
) -> String {
    let requested = if requested.trim().is_empty() {
        "file"
    } else {
        requested
    };
    if used_names.insert(requested.to_string()) {
        return requested.to_string();
    }

    let path = std::path::Path::new(requested);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|value| value.to_str());
    for suffix in 2..=10_000 {
        let candidate = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
    }
    format!("{}-{}", uuid::Uuid::new_v4(), requested)
}

pub async fn handle_file_download_proxy(
    Path(token): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let app_handle = &state.app_handle;
    update_activity(app_handle);
    let shared_state = app_handle.state::<SharedFileState>();

    let file_path = shared_state.0.lock().ok().and_then(|mut guard| {
        guard.retain(|_, entry| !entry.is_expired());
        guard.get(&token).map(|entry| entry.path.clone())
    });

    if let Some(path_str) = file_path {
        let path = std::path::PathBuf::from(&path_str);
        if path.exists() {
            let filename = params
                .get("name")
                .map(|name| safe_file_name(name))
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
            let disposition_filename: String = filename
                .chars()
                .filter(|ch| !ch.is_control())
                .map(|ch| if ch == '"' || ch == '\\' { '_' } else { ch })
                .collect();
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let is_image = mime.starts_with("image/");
            let is_video = mime.starts_with("video/");
            let encoded_name = urlencoding::encode(&filename);
            let disposition = if is_image || is_video {
                format!(
                    "inline; filename=\"{}\"; filename*=UTF-8''{}",
                    disposition_filename, encoded_name
                )
            } else {
                format!(
                    "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                    disposition_filename, encoded_name
                )
            };

            if let Ok(mut file) = File::open(&path).await {
                let metadata = match file.metadata().await {
                    Ok(m) => m,
                    Err(_) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Metadata failed")
                            .into_response()
                    }
                };
                let total_size = metadata.len();
                if let Some(range) = headers.get(header::RANGE).and_then(|h| h.to_str().ok()) {
                    let Some((start, end)) = parse_single_byte_range(range, total_size) else {
                        return (
                            StatusCode::RANGE_NOT_SATISFIABLE,
                            [(header::CONTENT_RANGE, format!("bytes */{}", total_size))],
                            Body::empty(),
                        )
                            .into_response();
                    };
                    let content_length = end - start + 1;

                    if file.seek(SeekFrom::Start(start)).await.is_ok() {
                        let stream =
                            ReaderStream::with_capacity(file.take(content_length), 64 * 1024);
                        let body = Body::from_stream(stream);

                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [
                                (header::CONTENT_TYPE, mime),
                                (header::CONTENT_DISPOSITION, disposition),
                                (header::ACCEPT_RANGES, "bytes".to_string()),
                                (
                                    header::CONTENT_RANGE,
                                    format!("bytes {}-{}/{}", start, end, total_size),
                                ),
                                (header::CONTENT_LENGTH, content_length.to_string()),
                            ],
                            body,
                        )
                            .into_response();
                    }
                }

                let stream = ReaderStream::with_capacity(file, 64 * 1024);
                let body = Body::from_stream(stream);

                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime),
                        (header::CONTENT_DISPOSITION, disposition),
                        (header::ACCEPT_RANGES, "bytes".to_string()),
                        (header::CONTENT_LENGTH, total_size.to_string()),
                    ],
                    body,
                )
                    .into_response();
            }
        }
    }

    (StatusCode::NOT_FOUND, "File not found").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_cannot_escape_the_receive_directory() {
        assert_eq!(safe_file_name("../../private/secret.txt"), "secret.txt");
        assert_eq!(safe_file_name(r"..\..\private\secret.txt"), "secret.txt");
        assert_eq!(safe_file_name("../.."), "unnamed-file");
    }

    #[test]
    fn upload_ids_are_path_safe() {
        assert!(safe_upload_id("mobile_123-abc").is_some());
        assert!(safe_upload_id("../../escape").is_none());
        assert!(safe_upload_id("").is_none());
    }

    #[test]
    fn request_tokens_support_qr_cookie_and_desktop_header() {
        let query_request = Request::builder()
            .uri("/?auth=session-123")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            request_token(&query_request).as_deref(),
            Some("session-123")
        );

        let cookie_request = Request::builder()
            .header(header::COOKIE, "other=x; tiez_ft_session=session-456")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            request_token(&cookie_request).as_deref(),
            Some("session-456")
        );

        let header_request = Request::builder()
            .header(header::AUTHORIZATION, "Bearer session-789")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            request_token(&header_request).as_deref(),
            Some("session-789")
        );
    }

    #[test]
    fn chunk_metadata_must_match_the_real_payload() {
        let metadata = ChunkMetadata {
            upload_id: "upload-1".to_string(),
            chunk_index: 0,
            total_chunks: 1,
            file_name: "small.txt".to_string(),
            sender_id: "mobile".to_string(),
            sender_name: "phone".to_string(),
            total_size: 4,
            content_type: Some("text/plain".to_string()),
            batch_id: None,
            batch_name: None,
            batch_index: None,
            batch_total: None,
            batch_size: None,
        };
        assert!(validate_chunk(&metadata, 4).is_ok());
        assert!(validate_chunk(&metadata, 3).is_err());

        let large_size = 25_u64 * 1024 * 1024 * 1024;
        let large_metadata = ChunkMetadata {
            upload_id: "large-upload".to_string(),
            chunk_index: 0,
            total_chunks: (large_size / TRANSFER_CHUNK_SIZE as u64) as usize,
            file_name: "large.bin".to_string(),
            sender_id: "mobile".to_string(),
            sender_name: "phone".to_string(),
            total_size: large_size,
            content_type: Some("application/octet-stream".to_string()),
            batch_id: None,
            batch_name: None,
            batch_index: None,
            batch_total: None,
            batch_size: None,
        };
        assert!(validate_chunk(&large_metadata, TRANSFER_CHUNK_SIZE).is_ok());
    }

    #[test]
    fn batch_archives_stream_files_and_preserve_duplicate_names() {
        let root = std::env::temp_dir().join(format!(
            "tiez-batch-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let first_dir = root.join("first");
        let second_dir = root.join("second");
        std::fs::create_dir_all(&first_dir).unwrap();
        std::fs::create_dir_all(&second_dir).unwrap();
        let first = first_dir.join("same.txt");
        let second = second_dir.join("same.txt");
        std::fs::write(&first, b"first").unwrap();
        std::fs::write(&second, b"second").unwrap();
        let archive = root.join("package.zip");

        create_batch_archive(&archive, &[first, second]).unwrap();

        let file = std::fs::File::open(&archive).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        assert_eq!(zip.len(), 2);
        assert_eq!(zip.by_index(0).unwrap().name(), "same.txt");
        assert_eq!(zip.by_index(1).unwrap().name(), "same (2).txt");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn byte_ranges_handle_open_ended_suffix_and_invalid_inputs() {
        assert_eq!(parse_single_byte_range("bytes=10-19", 100), Some((10, 19)));
        assert_eq!(parse_single_byte_range("bytes=90-", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=-200", 100), Some((0, 99)));
        assert_eq!(parse_single_byte_range("bytes=90-200", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=20-10", 100), None);
        assert_eq!(parse_single_byte_range("bytes=100-", 100), None);
        assert_eq!(parse_single_byte_range("bytes=0-1,4-5", 100), None);
        assert_eq!(parse_single_byte_range("bytes=0-", 0), None);
    }
}
