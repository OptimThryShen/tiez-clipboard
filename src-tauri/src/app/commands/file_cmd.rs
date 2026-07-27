use crate::app_state::AppDataDir;
use crate::error::{AppError, AppResult};
use base64::Engine;
use image::ImageFormat;
use reqwest::header;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tauri::State;
use urlencoding::decode;

#[derive(Serialize)]
pub struct FileSize {
    pub size: u64,
}

#[tauri::command]
pub fn get_file_size(path: String) -> AppResult<FileSize> {
    use std::fs;
    let metadata = fs::metadata(&path).map_err(AppError::from)?;
    Ok(FileSize {
        size: metadata.len(),
    })
}

#[tauri::command]
pub async fn save_file_copy(source_path: String, target_path: String) -> AppResult<()> {
    std::fs::copy(source_path, target_path).map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn save_file_batch(
    source_paths: Vec<String>,
    target_dir: String,
    batch_name: String,
) -> AppResult<String> {
    if source_paths.len() < 2 || source_paths.len() > 2_000 {
        return Err(AppError::Validation(
            "文件包必须包含 2–2000 个文件".to_string(),
        ));
    }
    let target_root = PathBuf::from(target_dir);
    if !target_root.is_dir() {
        return Err(AppError::Validation("目标文件夹无效".to_string()));
    }

    tauri::async_runtime::spawn_blocking(move || {
        let safe_batch_name = sanitize_copy_name(&batch_name, "TieZ 文件包");
        let package_dir = unique_copy_path(&target_root, &safe_batch_name, true);
        std::fs::create_dir(&package_dir).map_err(AppError::from)?;

        for source in source_paths {
            let source = PathBuf::from(source);
            if !source.is_file() {
                let _ = std::fs::remove_dir_all(&package_dir);
                return Err(AppError::Validation(format!(
                    "源文件不存在：{}",
                    source.to_string_lossy()
                )));
            }
            let requested_name = source
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("文件");
            let safe_name = sanitize_copy_name(requested_name, "文件");
            let target = unique_copy_path(&package_dir, &safe_name, false);
            if let Err(error) = std::fs::copy(&source, &target) {
                let _ = std::fs::remove_dir_all(&package_dir);
                return Err(AppError::from(error));
            }
        }

        Ok(package_dir.to_string_lossy().to_string())
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
}

fn sanitize_copy_name(value: &str, fallback: &str) -> String {
    let cleaned = value
        .chars()
        .filter(|character| !character.is_control())
        .map(|character| {
            if matches!(character, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                character
            }
        })
        .take(120)
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

fn unique_copy_path(root: &Path, requested_name: &str, directory: bool) -> PathBuf {
    let initial = root.join(requested_name);
    if !initial.exists() {
        return initial;
    }
    let requested = Path::new(requested_name);
    let stem = requested
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("文件");
    let extension = if directory {
        None
    } else {
        requested.extension().and_then(|value| value.to_str())
    };
    for suffix in 2..=10_000 {
        let candidate_name = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        let candidate = root.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("{}-{}", stem, uuid::Uuid::new_v4()))
}

fn normalize_image_ext(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "webp" => Some("webp"),
        "gif" => Some("gif"),
        "avif" => Some("avif"),
        _ => None,
    }
}

pub(crate) fn image_ext_from_filename(name: &str) -> Option<&'static str> {
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    normalize_image_ext(ext)
}

pub(crate) fn image_ext_from_bytes(bytes: &[u8]) -> Option<&'static str> {
    let format = image::guess_format(bytes).ok()?;
    match format {
        ImageFormat::Png => Some("png"),
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::Gif => Some("gif"),
        ImageFormat::WebP => Some("webp"),
        _ => None,
    }
}

pub(crate) fn image_ext_from_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/avif" => Some("avif"),
        _ => None,
    }
}

fn image_ext_from_url(url: &reqwest::Url) -> Option<&'static str> {
    let path = url.path();
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    normalize_image_ext(ext)
}

fn normalize_emoji_source_path(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return Err(AppError::Validation("source_path is empty".to_string()));
    }

    let mut normalized = if trimmed.starts_with("file://") {
        let path_raw = trimmed.trim_start_matches("file://");
        let decoded = decode(path_raw)
            .map(|p| p.into_owned())
            .unwrap_or_else(|_| path_raw.to_string());

        if decoded.chars().nth(1) == Some(':') {
            decoded
        } else if decoded.starts_with('/') && decoded.chars().nth(2) == Some(':') {
            decoded[1..].to_string()
        } else if !decoded.starts_with('/') && !decoded.starts_with('\\') {
            format!("//{}", decoded)
        } else {
            decoded
        }
    } else {
        decode(trimmed)
            .map(|p| p.into_owned())
            .unwrap_or_else(|_| trimmed.to_string())
    };

    normalized = normalized
        .split('?')
        .next()
        .unwrap_or(&normalized)
        .split('#')
        .next()
        .unwrap_or(&normalized)
        .to_string();

    if normalized.is_empty() {
        return Err(AppError::Validation("source_path is empty".to_string()));
    }

    Ok(normalized)
}

pub(crate) fn save_emoji_favorite_bytes_to_dir(
    data_dir: &Path,
    bytes: &[u8],
    ext: &str,
) -> AppResult<String> {
    let ext = normalize_image_ext(ext)
        .ok_or_else(|| AppError::Validation("unsupported file type".to_string()))?;

    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();

    let favorites_dir = data_dir.join("emoji_favorites");
    if !favorites_dir.exists() {
        std::fs::create_dir_all(&favorites_dir).map_err(AppError::from)?;
    }

    let file_name = format!("fav_{:x}.{}", hash, ext);
    let target_path = favorites_dir.join(file_name);
    if !target_path.exists() {
        std::fs::write(&target_path, bytes).map_err(AppError::from)?;
    }

    Ok(target_path.to_string_lossy().to_string())
}

pub(crate) fn list_emoji_favorite_paths_in_dir(data_dir: &std::path::Path) -> AppResult<Vec<String>> {
    let favorites_dir = data_dir.join("emoji_favorites");
    if !favorites_dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&favorites_dir).map_err(AppError::from)? {
        let path = entry.map_err(AppError::from)?.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if normalize_image_ext(ext).is_some() {
            paths.push(path.to_string_lossy().to_string());
        }
    }

    paths.sort();
    Ok(paths)
}

#[tauri::command]
pub fn list_emoji_favorites(app_data: State<'_, AppDataDir>) -> AppResult<Vec<String>> {
    let data_dir = app_data.0.lock().unwrap().clone();
    list_emoji_favorite_paths_in_dir(&data_dir)
}

#[tauri::command]
pub async fn save_emoji_favorite(
    app_data: State<'_, AppDataDir>,
    source_path: String,
) -> AppResult<String> {
    let source_path = normalize_emoji_source_path(&source_path)?;

    let ext = match image_ext_from_filename(&source_path) {
        Some(ext) => ext,
        None => {
            return Err(AppError::Validation("unsupported file type".to_string()));
        }
    };

    let bytes = std::fs::read(&source_path).map_err(AppError::from)?;

    let data_dir = app_data.0.lock().unwrap().clone();
    save_emoji_favorite_bytes_to_dir(&data_dir, &bytes, ext)
}

#[tauri::command]
pub async fn remove_emoji_favorite(app_data: State<'_, AppDataDir>, path: String) -> AppResult<()> {
    if path.trim().is_empty() {
        return Ok(());
    }

    let data_dir = app_data.0.lock().unwrap().clone();
    let favorites_dir = data_dir.join("emoji_favorites");
    let favorites_dir = favorites_dir.canonicalize().unwrap_or(favorites_dir);

    let target_path = std::path::PathBuf::from(&path);
    if let Ok(target_canonical) = target_path.canonicalize() {
        if target_canonical.starts_with(&favorites_dir) && target_canonical.is_file() {
            let _ = std::fs::remove_file(target_canonical);
        }
    } else if target_path.starts_with(&favorites_dir) && target_path.is_file() {
        let _ = std::fs::remove_file(target_path);
    }

    Ok(())
}

#[tauri::command]
pub async fn save_emoji_favorite_data_url(
    app_data: State<'_, AppDataDir>,
    data_url: String,
    file_name: Option<String>,
) -> AppResult<String> {
    let (mime, payload) = if data_url.starts_with("data:") {
        let mut parts = data_url.splitn(2, ',');
        let header = parts.next().unwrap_or("");
        let payload = parts.next().unwrap_or("");
        let mime = header
            .trim_start_matches("data:")
            .split(';')
            .next()
            .unwrap_or("");
        (mime.to_string(), payload.to_string())
    } else {
        ("".to_string(), data_url)
    };

    if payload.is_empty() {
        return Err(AppError::Validation("data_url is empty".to_string()));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| AppError::Internal(format!("Base64 decode failed: {}", e)))?;

    let ext = file_name
        .as_deref()
        .and_then(image_ext_from_filename)
        .or_else(|| image_ext_from_mime(mime.as_str()))
        .or_else(|| image_ext_from_bytes(&bytes))
        .unwrap_or("png");

    let data_dir = app_data.0.lock().unwrap().clone();
    save_emoji_favorite_bytes_to_dir(&data_dir, &bytes, ext)
}

pub(crate) async fn save_emoji_favorite_url_to_dir(
    data_dir: PathBuf,
    url: String,
) -> AppResult<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("url is empty".to_string()));
    }
    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| AppError::Validation("invalid url".to_string()))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::Validation("unsupported url scheme".to_string()));
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| {
            AppError::Network(e.to_string())
        })?;

    let response = client
        .get(parsed.clone())
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "HTTP {} when downloading image",
            response.status()
        )));
    }

    let mime = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let mime = mime.split(';').next().unwrap_or("").trim().to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    if bytes.is_empty() {
        return Err(AppError::Validation("empty image response".to_string()));
    }

    let ext = image_ext_from_mime(mime.as_str())
        .or_else(|| image_ext_from_url(&parsed))
        .or_else(|| image_ext_from_bytes(&bytes))
        .ok_or_else(|| AppError::Validation("unsupported image type".to_string()))?;

    save_emoji_favorite_bytes_to_dir(&data_dir, &bytes, ext)
}

#[tauri::command]
pub async fn save_emoji_favorite_url(
    app_data: State<'_, AppDataDir>,
    url: String,
) -> AppResult<String> {
    let data_dir = app_data.0.lock().unwrap().clone();
    save_emoji_favorite_url_to_dir(data_dir, url).await
}
