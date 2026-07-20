//! Local-at-rest encryption for sensitive clipboard / settings values.
//!
//! Format: `enc1:` + base64(nonce || ciphertext)  (AES-256-GCM)
//! Legacy: `plain:` + plaintext  (migrated on read / startup)
//!
//! Master key is stored in macOS Keychain or Windows DPAPI-protected file.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use rand::RngCore;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Current ciphertext prefix (AES-GCM).
pub const ENCRYPT_PREFIX: &str = "enc1:";
/// Legacy fake-encryption prefix from earlier builds.
pub const LEGACY_PLAIN_PREFIX: &str = "plain:";

#[cfg(all(target_os = "macos", not(debug_assertions)))]
const KEYCHAIN_SERVICE: &str = "com.tiez.clipboard";
#[cfg(all(target_os = "macos", not(debug_assertions)))]
const KEYCHAIN_ACCOUNT: &str = "master-key-v1";
#[cfg(target_os = "windows")]
const DPAPI_KEY_FILE: &str = "master.key.dpapi";
/// Dev / non-OS-keystore fallback (0600 file under app data dir).
#[cfg(any(
    not(any(target_os = "macos", target_os = "windows")),
    all(target_os = "macos", debug_assertions)
))]
const FALLBACK_KEY_FILE: &str = "master.key";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

// Option so a failed load (e.g. user denied Keychain access) is cached and we
// never re-prompt within the same session.
static MASTER_KEY: OnceLock<Option<[u8; KEY_LEN]>> = OnceLock::new();
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// True if value is protected (modern or legacy wrapper).
pub fn is_encrypted_value(value: &str) -> bool {
    value.starts_with(ENCRYPT_PREFIX) || value.starts_with(LEGACY_PLAIN_PREFIX)
}

/// True if value uses the real AES-GCM format.
pub fn is_modern_ciphertext(value: &str) -> bool {
    value.starts_with(ENCRYPT_PREFIX)
}

/// Initialize encryption with the app data directory (call once at startup).
///
/// On macOS release builds, Keychain access is deferred to a background thread
/// so the auth dialog cannot deadlock the Tauri setup / main run loop.
pub fn init(data_dir: &Path) {
    let _ = DATA_DIR.set(data_dir.to_path_buf());

    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    {
        // Prewarm off the main thread: SecurityAgent UI needs a free run loop.
        std::thread::spawn(|| {
            let _ = get_master_key();
        });
    }

    #[cfg(not(all(target_os = "macos", not(debug_assertions))))]
    {
        let _ = get_master_key();
    }
}

fn get_master_key() -> Option<&'static [u8; KEY_LEN]> {
    MASTER_KEY
        .get_or_init(|| match load_or_create_master_key() {
            Ok(key) => Some(key),
            Err(e) => {
                eprintln!(">>> [ENCRYPTION] master key unavailable this session: {e}");
                None
            }
        })
        .as_ref()
}

#[allow(dead_code)]
fn data_dir() -> Option<&'static PathBuf> {
    DATA_DIR.get()
}

fn load_or_create_master_key() -> Result<[u8; KEY_LEN], String> {
    // Debug macOS: never touch Keychain. Dev binaries re-sign every rebuild, so
    // Keychain prompts loop / deadlock the main thread during Tauri setup.
    #[cfg(all(target_os = "macos", debug_assertions))]
    {
        eprintln!(
            ">>> [ENCRYPTION] debug build: using local master.key (Keychain disabled)"
        );
        return load_or_create_file_key(FALLBACK_KEY_FILE);
    }

    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    {
        match load_key_from_keychain() {
            Ok(key) => return Ok(key),
            // Only create a fresh key when the item truly does not exist.
            // Access-denied / user-cancel must NOT overwrite the existing key,
            // otherwise previously encrypted data becomes undecryptable.
            Err(KeychainLoadError::NotFound) => {
                let key = generate_key();
                store_key_in_keychain(&key)?;
                return Ok(key);
            }
            Err(KeychainLoadError::Other(msg)) => return Err(msg),
        }
    }

    #[cfg(target_os = "windows")]
    {
        let dir = data_dir().ok_or_else(|| "encryption data dir not set".to_string())?;
        let path = dir.join(DPAPI_KEY_FILE);
        if path.exists() {
            return load_key_dpapi(&path);
        }
        let key = generate_key();
        store_key_dpapi(&path, &key)?;
        return Ok(key);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        load_or_create_file_key(FALLBACK_KEY_FILE)
    }
}

#[cfg(any(
    not(any(target_os = "macos", target_os = "windows")),
    all(target_os = "macos", debug_assertions)
))]
fn load_or_create_file_key(filename: &str) -> Result<[u8; KEY_LEN], String> {
    let dir = data_dir().ok_or_else(|| "encryption data dir not set".to_string())?;
    let path = dir.join(filename);
    if path.exists() {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() != KEY_LEN {
            return Err("invalid fallback key length".into());
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }
    let key = generate_key();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, key).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

fn generate_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
enum KeychainLoadError {
    NotFound,
    Other(String),
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
fn load_key_from_keychain() -> Result<[u8; KEY_LEN], KeychainLoadError> {
    use security_framework::passwords::get_generic_password;

    // errSecItemNotFound = -25300
    const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

    let bytes = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).map_err(|e| {
        if e.code() == ERR_SEC_ITEM_NOT_FOUND {
            KeychainLoadError::NotFound
        } else {
            KeychainLoadError::Other(format!("keychain get: {e}"))
        }
    })?;
    if bytes.len() != KEY_LEN {
        return Err(KeychainLoadError::Other(
            "invalid keychain key length".into(),
        ));
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
fn store_key_in_keychain(key: &[u8; KEY_LEN]) -> Result<(), String> {
    use security_framework::passwords::set_generic_password;
    set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, key)
        .map_err(|e| format!("keychain set: {e}"))
}

#[cfg(target_os = "windows")]
fn store_key_dpapi(path: &Path, key: &[u8; KEY_LEN]) -> Result<(), String> {
    let protected = dpapi_protect(key)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, protected).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn load_key_dpapi(path: &Path) -> Result<[u8; KEY_LEN], String> {
    let protected = std::fs::read(path).map_err(|e| e.to_string())?;
    let plain = dpapi_unprotect(&protected)?;
    if plain.len() != KEY_LEN {
        return Err("invalid DPAPI key length".into());
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&plain);
    Ok(key)
}

#[cfg(target_os = "windows")]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPTOAPI_BLOB};

    unsafe {
        let mut input = CRYPTOAPI_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPTOAPI_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        CryptProtectData(
            &mut input,
            windows::core::PCWSTR::null(),
            None,
            None,
            None,
            0,
            &mut output,
        )
        .map_err(|e| format!("CryptProtectData: {e}"))?;

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        if !output.pbData.is_null() {
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        }
        Ok(result)
    }
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPTOAPI_BLOB};

    unsafe {
        let mut input = CRYPTOAPI_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPTOAPI_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            0,
            &mut output,
        )
        .map_err(|e| format!("CryptUnprotectData: {e}"))?;

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        if !output.pbData.is_null() {
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        }
        Ok(result)
    }
}

pub fn encrypt_value(plain: &str) -> Option<String> {
    #[cfg(feature = "portable")]
    {
        let _ = plain;
        return None;
    }
    #[cfg(not(feature = "portable"))]
    {
        let key = get_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(key).ok()?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plain.as_bytes()).ok()?;
        let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ciphertext);
        let encoded = base64::engine::general_purpose::STANDARD.encode(packed);
        Some(format!("{ENCRYPT_PREFIX}{encoded}"))
    }
}

pub fn decrypt_value(cipher: &str) -> Option<String> {
    if let Some(payload) = cipher.strip_prefix(LEGACY_PLAIN_PREFIX) {
        return Some(payload.to_string());
    }

    let Some(payload) = cipher.strip_prefix(ENCRYPT_PREFIX) else {
        // Unprefixed plaintext (or unknown format)
        return Some(cipher.to_string());
    };

    #[cfg(feature = "portable")]
    {
        let _ = payload;
        return Some(cipher.to_string());
    }

    #[cfg(not(feature = "portable"))]
    {
        let key = get_master_key()?;
        let packed = base64::engine::general_purpose::STANDARD
            .decode(payload.as_bytes())
            .ok()?;
        if packed.len() <= NONCE_LEN {
            return None;
        }
        let (nonce_bytes, ciphertext) = packed.split_at(NONCE_LEN);
        let aes = Aes256Gcm::new_from_slice(key).ok()?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = aes.decrypt(nonce, ciphertext).ok()?;
        String::from_utf8(plain).ok()
    }
}

/// Re-encrypt any `plain:` blobs found in settings + clipboard_history.
pub fn migrate_legacy_ciphertexts(conn: &rusqlite::Connection) -> Result<usize, String> {
    let mut upgraded = 0usize;

    // Settings: sensitive keys with legacy or plaintext values are rewritten via encrypt.
    {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;

        let mut updates: Vec<(String, String)> = Vec::new();
        for row in rows {
            let (key, value) = row.map_err(|e| e.to_string())?;
            if !crate::database::is_sensitive_key(&key) {
                continue;
            }
            if is_modern_ciphertext(&value) {
                continue;
            }
            let plain = decrypt_value(&value).unwrap_or(value);
            if let Some(enc) = encrypt_value(&plain) {
                updates.push((key, enc));
            }
        }
        for (key, enc) in updates {
            conn.execute(
                "UPDATE settings SET value = ?1 WHERE key = ?2",
                rusqlite::params![enc, key],
            )
            .map_err(|e| e.to_string())?;
            upgraded += 1;
        }
    }

    // Clipboard text fields with legacy plain: prefix
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, content, preview, html_content FROM clipboard_history
                 WHERE content LIKE 'plain:%'
                    OR preview LIKE 'plain:%'
                    OR html_content LIKE 'plain:%'",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut updates: Vec<(i64, String, String, Option<String>)> = Vec::new();
        for row in rows {
            let (id, content, preview, html) = row.map_err(|e| e.to_string())?;
            let new_content = upgrade_field(&content);
            let new_preview = upgrade_field(&preview);
            let new_html = html.as_ref().map(|h| upgrade_field(h));
            if new_content != content
                || new_preview != preview
                || new_html.as_ref() != html.as_ref()
            {
                updates.push((id, new_content, new_preview, new_html));
            }
        }
        for (id, content, preview, html) in updates {
            conn.execute(
                "UPDATE clipboard_history SET content = ?1, preview = ?2, html_content = ?3 WHERE id = ?4",
                rusqlite::params![content, preview, html, id],
            )
            .map_err(|e| e.to_string())?;
            upgraded += 1;
        }
    }

    Ok(upgraded)
}

fn upgrade_field(value: &str) -> String {
    if is_modern_ciphertext(value) {
        return value.to_string();
    }
    if value.starts_with(LEGACY_PLAIN_PREFIX) {
        let plain = decrypt_value(value).unwrap_or_else(|| value.to_string());
        return encrypt_value(&plain).unwrap_or(plain);
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_plain_decrypts() {
        assert_eq!(decrypt_value("plain:hello").as_deref(), Some("hello"));
        assert!(is_encrypted_value("plain:hello"));
        assert!(!is_modern_ciphertext("plain:hello"));
    }
}
