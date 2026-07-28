use rusqlite::{params, Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Migration 1: Initial Baseline
    if current_version < 1 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY,
                content_type TEXT NOT NULL,
                content TEXT NOT NULL,
                source_app TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                preview TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (1)", [])?;
    }

    // Migration 2: Add core feature columns
    if current_version < 2 {
        let columns = [
            ("is_pinned", "INTEGER NOT NULL DEFAULT 0"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("use_count", "INTEGER NOT NULL DEFAULT 0"),
            ("pinned_order", "INTEGER NOT NULL DEFAULT 0"),
            ("content_hash", "INTEGER NOT NULL DEFAULT 0"),
            ("html_content", "TEXT"),
        ];

        for (name, def) in columns {
            if !has_column(conn, "clipboard_history", name)? {
                conn.execute(
                    &format!("ALTER TABLE clipboard_history ADD COLUMN {} {}", name, def),
                    [],
                )?;
            }
        }
        conn.execute("INSERT INTO schema_migrations (version) VALUES (2)", [])?;
    }

    // Migration 3: Add is_external
    if current_version < 3 {
        if !has_column(conn, "clipboard_history", "is_external")? {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN is_external INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        conn.execute("INSERT INTO schema_migrations (version) VALUES (3)", [])?;
    }

    // Migration 4: Tag management
    if current_version < 4 {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS saved_tags (
                name TEXT PRIMARY KEY,
                color TEXT
            )",
            [],
        )?;

        // Insert default tags
        let _ = conn.execute(
            "INSERT OR IGNORE INTO saved_tags (name) VALUES ('sensitive')",
            [],
        );
        let _ = conn.execute(
            "INSERT OR IGNORE INTO saved_tags (name) VALUES ('密码')",
            [],
        );

        conn.execute("INSERT INTO schema_migrations (version) VALUES (4)", [])?;
    }

    // Migration 5: Performance indexes
    if current_version < 5 {
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_pinned_order_time
                ON clipboard_history (is_pinned, pinned_order, timestamp);
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_type_hash
                ON clipboard_history (content_type, content_hash);
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_timestamp
                ON clipboard_history (timestamp);
        ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (5)", [])?;
    }

    // Migration 6: Normalize tags into entry_tags
    if current_version < 6 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS entry_tags (
                entry_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_entry_tags_tag ON entry_tags (tag);
            CREATE INDEX IF NOT EXISTS idx_entry_tags_entry ON entry_tags (entry_id);
        ",
        )?;

        // Backfill entry_tags from clipboard_history.tags JSON
        conn.execute("BEGIN", [])?;
        let backfill = (|| -> Result<()> {
            let mut stmt = conn.prepare("SELECT id, tags FROM clipboard_history")?;
            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let tags: Option<String> = row.get(1)?;
                Ok((id, tags.unwrap_or_else(|| "[]".to_string())))
            })?;

            for row in rows {
                let (id, tags_json) = row?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                for tag in tags {
                    if tag.trim().is_empty() {
                        continue;
                    }
                    conn.execute(
                        "INSERT OR IGNORE INTO entry_tags (entry_id, tag) VALUES (?1, ?2)",
                        params![id, tag],
                    )?;
                }
            }
            Ok(())
        })();

        if let Err(err) = backfill {
            let _ = conn.execute("ROLLBACK", []);
            return Err(err);
        }
        conn.execute("COMMIT", [])?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (6)", [])?;
    }

    // Migration 7: Cloud sync tombstones for deletion propagation
    if current_version < 7 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS cloud_sync_tombstones (
                content_type TEXT NOT NULL,
                content_hash INTEGER NOT NULL,
                deleted_at INTEGER NOT NULL,
                PRIMARY KEY (content_type, content_hash)
            );
            CREATE INDEX IF NOT EXISTS idx_cloud_sync_tombstones_deleted_at
                ON cloud_sync_tombstones (deleted_at);
            ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (7)", [])?;
    }

    // Migration 8: Local incremental sync index (for delta diff against last uploaded state)
    if current_version < 8 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS cloud_sync_local_index (
                sync_key TEXT PRIMARY KEY,
                digest TEXT NOT NULL
            );
            ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (8)", [])?;
    }

    // Migration 9: Persist source app path for source icon rendering
    if current_version < 9 {
        if !has_column(conn, "clipboard_history", "source_app_path")? {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN source_app_path TEXT",
                [],
            )?;
        }
        conn.execute("INSERT INTO schema_migrations (version) VALUES (9)", [])?;
    }

    // Migration 10: Cloud sync content type preferences (per-device; not synced in settings snapshot)
    if current_version < 10 {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('cloud_sync_content_prefs', '{\"text\":true,\"image\":true,\"file_path\":true,\"emoji\":true}')",
            [],
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (10)", [])?;
    }

    // Migration 11: Per-entry free-text note (separate from category tags)
    if current_version < 11 {
        if !has_column(conn, "clipboard_history", "note")? {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN note TEXT NOT NULL DEFAULT ''",
                [],
            )?;
            eprintln!("[migrations] added clipboard_history.note column");
        }
        conn.execute("INSERT INTO schema_migrations (version) VALUES (11)", [])?;
    }

    // Safety net: older builds may have queried `note` before migration 11 ran,
    // or schema_migrations can drift. Always ensure the column exists.
    if !has_column(conn, "clipboard_history", "note")? {
        conn.execute(
            "ALTER TABLE clipboard_history ADD COLUMN note TEXT NOT NULL DEFAULT ''",
            [],
        )?;
        eprintln!("[migrations] repaired missing clipboard_history.note column");
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (11)",
            [],
        )?;
    }

    // Migration 12: Cover the exact list ordering used by paginated history.
    // Including `id` also makes entries with identical timestamps deterministic.
    if current_version < 12 {
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_list_order
                ON clipboard_history (
                    is_pinned DESC,
                    pinned_order DESC,
                    timestamp DESC,
                    id DESC
                );
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_type_list_order
                ON clipboard_history (
                    content_type,
                    is_pinned DESC,
                    pinned_order DESC,
                    timestamp DESC,
                    id DESC
                );
            ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (12)", [])?;
    }

    // Migration 13: Structured per-entry shortcut and usage metadata.
    // These fields are intentionally separate from the user-facing note.
    if current_version < 13 {
        let columns = [
            ("item_hotkey", "TEXT NOT NULL DEFAULT ''"),
            ("item_hotkey_global", "INTEGER NOT NULL DEFAULT 0"),
            ("last_used_at", "INTEGER NOT NULL DEFAULT 0"),
            ("move_to_group_hotkey", "TEXT NOT NULL DEFAULT ''"),
            ("move_to_group_hotkey_global", "INTEGER NOT NULL DEFAULT 0"),
        ];
        for (name, definition) in columns {
            if !has_column(conn, "clipboard_history", name)? {
                conn.execute(
                    &format!("ALTER TABLE clipboard_history ADD COLUMN {name} {definition}"),
                    [],
                )?;
            }
        }
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_last_used_at
                ON clipboard_history (last_used_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_item_hotkey
                ON clipboard_history (item_hotkey)
                WHERE item_hotkey <> '';
            ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (13)", [])?;
    }

    // Migration 14: Split immutable creation time from mutable list ordering.
    // Existing `timestamp` values are the best available historical value, so
    // use them for both fields to preserve the exact pre-upgrade list order.
    if current_version < 14 {
        if !has_column(conn, "clipboard_history", "created_at")? {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if !has_column(conn, "clipboard_history", "sort_at")? {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN sort_at INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        conn.execute(
            "UPDATE clipboard_history SET
                created_at = CASE WHEN created_at <= 0 THEN timestamp ELSE created_at END,
                sort_at = CASE WHEN sort_at <= 0 THEN timestamp ELSE sort_at END",
            [],
        )?;
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_sort_order
                ON clipboard_history (
                    is_pinned DESC,
                    pinned_order DESC,
                    sort_at DESC,
                    id DESC
                );
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_type_sort_order
                ON clipboard_history (
                    content_type,
                    is_pinned DESC,
                    pinned_order DESC,
                    sort_at DESC,
                    id DESC
                );
            ",
        )?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (14)", [])?;
    }

    Ok(())
}

fn has_column(conn: &Connection, table_name: &str, column_name: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column_name {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_14_preserves_existing_order_and_backfills_creation_time() {
        let conn = Connection::open_in_memory().expect("open database");
        conn.execute_batch(
            "
            CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            INSERT INTO schema_migrations(version) VALUES (12);
            CREATE TABLE clipboard_history (
                id INTEGER PRIMARY KEY,
                content_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                pinned_order INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO clipboard_history(id, content_type, timestamp)
                VALUES
                    (1, 'text', 1700000000123),
                    (2, 'text', 1700000000456);
            ",
        )
        .expect("create version 12 fixture");

        run_migrations(&conn).expect("run update migrations");

        let values: (i64, i64, i64) = conn
            .query_row(
                "SELECT created_at, sort_at, last_used_at
                 FROM clipboard_history WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read migrated fields");
        assert_eq!(values, (1_700_000_000_123, 1_700_000_000_123, 0));
        let migrated_order: Vec<i64> = conn
            .prepare("SELECT id FROM clipboard_history ORDER BY sort_at DESC, id DESC")
            .expect("prepare migrated order query")
            .query_map([], |row| row.get(0))
            .expect("query migrated order")
            .collect::<Result<Vec<_>>>()
            .expect("collect migrated order");
        assert_eq!(migrated_order, vec![2, 1]);
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("read schema version");
        assert_eq!(version, 14);
    }
}
