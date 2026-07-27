use crate::database::is_sensitive_key;
use crate::infrastructure::encryption;
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait SettingsRepository {
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn get_all(&self) -> Result<HashMap<String, String>>;
    fn clear(&self) -> Result<()>;
}

pub struct SqliteSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSettingsRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn get_raw(conn: &Connection, key: &str) -> Result<Option<String>> {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            if is_sensitive_key(key) && encryption::is_encrypted_value(&value) {
                return Ok(Some(Self::decrypt_sensitive_value(key, &value)));
            }
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn maybe_encrypt(&self, key: &str, value: &str) -> String {
        #[cfg(feature = "portable")]
        let _ = key;
        #[cfg(not(feature = "portable"))]
        {
            if is_sensitive_key(key) && !encryption::is_modern_ciphertext(value) {
                let plain = if encryption::is_encrypted_value(value) {
                    encryption::decrypt_value(value).unwrap_or_else(|| value.to_string())
                } else {
                    value.to_string()
                };
                return encryption::encrypt_value(&plain).unwrap_or(plain);
            }
        }
        value.to_string()
    }

    fn maybe_decrypt(&self, key: &str, value: &str) -> String {
        if is_sensitive_key(key) && encryption::is_encrypted_value(value) {
            return Self::decrypt_sensitive_value(key, value);
        }
        value.to_string()
    }

    fn decrypt_sensitive_value(key: &str, value: &str) -> String {
        match encryption::decrypt_value(value) {
            Some(plain) if plain != value || !encryption::is_modern_ciphertext(value) => plain,
            _ => {
                // Never expose ciphertext through commands such as `get_settings`.
                // A common cause in development is opening the production database
                // with a debug-only key; returning the raw value would also allow the
                // UI to accidentally save that ciphertext as the visible username.
                eprintln!(
                    ">>> [ENCRYPTION] unable to decrypt sensitive setting '{key}'; hiding its value"
                );
                String::new()
            }
        }
    }
}

impl SettingsRepository for SqliteSettingsRepository {
    fn set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let final_value = self.maybe_encrypt(key, value);

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![key, final_value],
        )?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            let decrypted = self.maybe_decrypt(key, &value);

            // Auto-migrate legacy plain: / plaintext sensitive values to enc1:
            #[cfg(not(feature = "portable"))]
            {
                if is_sensitive_key(key) && !encryption::is_modern_ciphertext(&value) {
                    let _ = conn.execute(
                        "UPDATE settings SET value = ? WHERE key = ?",
                        params![self.maybe_encrypt(key, &decrypted), key],
                    );
                }
            }

            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    fn get_all(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut settings = HashMap::new();
        for row in rows {
            let (key, value) = row?;
            let decrypted = self.maybe_decrypt(&key, &value);

            #[cfg(not(feature = "portable"))]
            {
                if is_sensitive_key(&key) && !encryption::is_modern_ciphertext(&value) {
                    let _ = conn.execute(
                        "UPDATE settings SET value = ? WHERE key = ?",
                        params![self.maybe_encrypt(&key, &decrypted), &key],
                    );
                }
            }

            settings.insert(key, decrypted);
        }
        Ok(settings)
    }

    fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM settings", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteSettingsRepository;

    #[test]
    fn undecryptable_sensitive_values_are_never_returned_to_the_ui() {
        assert_eq!(
            SqliteSettingsRepository::decrypt_sensitive_value(
                "mqtt_username",
                "enc1:not-valid-ciphertext"
            ),
            ""
        );
    }

    #[test]
    fn legacy_sensitive_values_still_decode() {
        assert_eq!(
            SqliteSettingsRepository::decrypt_sensitive_value(
                "mqtt_username",
                "plain:example-user"
            ),
            "example-user"
        );
    }
}
