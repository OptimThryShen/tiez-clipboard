mod ditto;
mod maccy;

use crate::domain::models::ClipboardEntry;
use crate::infrastructure::repository::clipboard_repo::{
    ClipboardRepository, SqliteClipboardRepository,
};
use crate::services::clipboard::build_entry_preview;
use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, Mutex};

const MAX_REPORT_WARNINGS: usize = 8;

#[derive(Debug)]
pub(super) struct ImportCandidate {
    pub content_type: String,
    pub content: String,
    pub html_content: Option<String>,
    pub created_at: i64,
    pub sort_at: i64,
    pub source_app: String,
    pub is_pinned: bool,
    pub pinned_order: i64,
    pub tags: Vec<String>,
    pub note: String,
    pub use_count: i32,
    pub item_hotkey: String,
    pub item_hotkey_global: bool,
    pub last_used_at: i64,
    pub move_to_group_hotkey: String,
    pub move_to_group_hotkey_global: bool,
    pub is_external: bool,
}

#[derive(Debug)]
pub(super) struct ImportedClip {
    pub candidates: Vec<ImportCandidate>,
    pub warning: Option<String>,
}

pub(super) trait ClipboardImporter {
    fn name(&self) -> &'static str;
    fn can_import(&self, path: &Path) -> Result<bool, String>;
    fn visit_clips(&self, path: &Path, visitor: &mut dyn FnMut(ImportedClip))
        -> Result<(), String>;
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub source: String,
    pub scanned: u64,
    pub imported: u64,
    pub duplicates: u64,
    pub unsupported: u64,
    pub failed: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredImportSource {
    pub source: String,
    pub path: String,
    pub modified_at: i64,
}

pub fn discover_clipboard_import_sources() -> Vec<DiscoveredImportSource> {
    let mut seen = std::collections::HashSet::new();
    let importers: Vec<Box<dyn ClipboardImporter>> = vec![
        Box::new(ditto::DittoImporter),
        Box::new(maccy::MaccyImporter),
    ];
    let mut discovered = Vec::new();
    for path in default_import_source_paths()
        .into_iter()
        .filter(|path| path.is_file())
    {
        let path_key = path.to_string_lossy().to_ascii_lowercase();
        if !seen.insert(path_key) {
            continue;
        }
        for importer in &importers {
            if importer.can_import(&path).unwrap_or(false) {
                let modified_at = path
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis() as i64)
                    .unwrap_or(0);
                discovered.push(DiscoveredImportSource {
                    source: importer.name().to_string(),
                    path: path.to_string_lossy().to_string(),
                    modified_at,
                });
                break;
            }
        }
    }
    discovered.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    discovered
}

#[cfg(target_os = "windows")]
fn default_import_source_paths() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    let mut add = |base: Option<std::ffi::OsString>, suffix: &[&str]| {
        if let Some(base) = base {
            let mut path = std::path::PathBuf::from(base);
            for part in suffix {
                path.push(part);
            }
            candidates.push(path);
        }
    };

    let roaming = std::env::var_os("APPDATA");
    let local = std::env::var_os("LOCALAPPDATA");
    let program_files = std::env::var_os("ProgramFiles");
    let program_files_x86 = std::env::var_os("ProgramFiles(x86)");
    let chocolatey = std::env::var_os("ChocolateyInstall");
    add(roaming, &["Ditto", "Ditto.db"]);
    add(local.clone(), &["Ditto_WindowsApp", "Ditto.db"]);
    add(local, &["Ditto_ChocolateyApp", "Ditto.db"]);
    add(program_files, &["Ditto", "Ditto.db"]);
    add(program_files_x86, &["Ditto", "Ditto.db"]);
    add(chocolatey, &["lib", "ditto", "tools", "Ditto.db"]);
    candidates
}

#[cfg(target_os = "macos")]
fn default_import_source_paths() -> Vec<std::path::PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return Vec::new();
    };
    vec![
        home.join(
            "Library/Containers/org.p0deje.Maccy/Data/Library/Application Support/Maccy/Storage.sqlite",
        ),
        home.join("Library/Application Support/Maccy/Storage.sqlite"),
    ]
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_import_source_paths() -> Vec<std::path::PathBuf> {
    Vec::new()
}

fn add_warning(report: &mut ImportReport, warning: String) {
    if report.warnings.len() < MAX_REPORT_WARNINGS && !report.warnings.contains(&warning) {
        report.warnings.push(warning);
    }
}

#[derive(Clone)]
struct StructuredImportMetadata {
    note: String,
    item_hotkey: String,
    item_hotkey_global: bool,
    last_used_at: i64,
    move_to_group_hotkey: String,
    move_to_group_hotkey_global: bool,
}

fn write_structured_metadata(
    destination: &Arc<Mutex<rusqlite::Connection>>,
    id: i64,
    metadata: &StructuredImportMetadata,
    merge_only: bool,
) -> Result<(), String> {
    let conn = destination.lock().map_err(|error| error.to_string())?;
    let sql = if merge_only {
        "UPDATE clipboard_history SET
            note = CASE WHEN TRIM(note) = '' THEN ?1 ELSE note END,
            item_hotkey = CASE WHEN item_hotkey = '' THEN ?2 ELSE item_hotkey END,
            item_hotkey_global = CASE
                WHEN item_hotkey = '' THEN ?3 ELSE item_hotkey_global END,
            last_used_at = MAX(last_used_at, ?4),
            move_to_group_hotkey = CASE
                WHEN move_to_group_hotkey = '' THEN ?5 ELSE move_to_group_hotkey END,
            move_to_group_hotkey_global = CASE
                WHEN move_to_group_hotkey = '' THEN ?6 ELSE move_to_group_hotkey_global END
         WHERE id = ?7"
    } else {
        "UPDATE clipboard_history SET
            item_hotkey = ?2,
            item_hotkey_global = ?3,
            last_used_at = ?4,
            move_to_group_hotkey = ?5,
            move_to_group_hotkey_global = ?6
         WHERE id = ?7"
    };
    conn.execute(
        sql,
        rusqlite::params![
            metadata.note,
            metadata.item_hotkey,
            i32::from(metadata.item_hotkey_global),
            metadata.last_used_at,
            metadata.move_to_group_hotkey,
            i32::from(metadata.move_to_group_hotkey_global),
            id
        ],
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

pub fn import_clipboard_data(
    destination: Arc<Mutex<rusqlite::Connection>>,
    data_dir: &Path,
    source_path: &Path,
) -> Result<ImportReport, String> {
    if !source_path.is_file() {
        return Err("所选文件不存在或不是普通文件".to_string());
    }

    let importers: Vec<Box<dyn ClipboardImporter>> = vec![
        Box::new(ditto::DittoImporter),
        Box::new(maccy::MaccyImporter),
    ];
    let mut selected = None;
    let mut probe_errors = Vec::new();
    for importer in importers {
        match importer.can_import(source_path) {
            Ok(true) => {
                selected = Some(importer);
                break;
            }
            Ok(false) => {}
            Err(error) => probe_errors.push(error),
        }
    }

    let importer = selected.ok_or_else(|| {
        if probe_errors.is_empty() {
            "无法识别该剪贴板数据库。目前支持 Ditto 数据库/导出文件和 Maccy Storage.sqlite。"
                .to_string()
        } else {
            format!("无法读取剪贴板数据库：{}", probe_errors.join("；"))
        }
    })?;

    let repo = SqliteClipboardRepository::new(destination.clone());
    let mut report = ImportReport {
        source: importer.name().to_string(),
        ..ImportReport::default()
    };

    importer.visit_clips(source_path, &mut |clip| {
        report.scanned += 1;
        if let Some(warning) = clip.warning {
            add_warning(&mut report, warning);
        }
        if clip.candidates.is_empty() {
            report.unsupported += 1;
            return;
        }

        for candidate in clip.candidates {
            let metadata = StructuredImportMetadata {
                note: candidate.note.clone(),
                item_hotkey: candidate.item_hotkey.clone(),
                item_hotkey_global: candidate.item_hotkey_global,
                last_used_at: candidate.last_used_at,
                move_to_group_hotkey: candidate.move_to_group_hotkey.clone(),
                move_to_group_hotkey_global: candidate.move_to_group_hotkey_global,
            };
            let duplicate = repo
                .get_entry_by_content(&candidate.content, Some(&candidate.content_type))
                .unwrap_or(None);
            if let Some(existing_id) = duplicate {
                if let Err(error) =
                    write_structured_metadata(&destination, existing_id, &metadata, true)
                {
                    add_warning(&mut report, format!("重复条目的迁移字段合并失败：{error}"));
                }
                report.duplicates += 1;
                continue;
            }

            let preview = build_entry_preview(
                &candidate.content_type,
                &candidate.content,
                candidate.html_content.as_deref(),
            );
            let entry = ClipboardEntry {
                id: 0,
                content_type: candidate.content_type,
                content: candidate.content,
                html_content: candidate.html_content,
                source_app: if candidate.source_app.trim().is_empty() {
                    importer.name().to_string()
                } else {
                    candidate.source_app
                },
                source_app_path: None,
                timestamp: candidate.sort_at,
                created_at: candidate.created_at,
                last_used_at: candidate.last_used_at,
                sort_at: candidate.sort_at,
                preview,
                is_pinned: candidate.is_pinned,
                tags: candidate.tags,
                note: candidate.note,
                use_count: candidate.use_count,
                is_external: candidate.is_external,
                pinned_order: candidate.pinned_order,
                file_preview_exists: true,
            };

            match repo.save(&entry, Some(data_dir)) {
                Ok(id) => {
                    let metadata_result =
                        write_structured_metadata(&destination, id, &metadata, false);
                    if let Err(error) = metadata_result {
                        report.failed += 1;
                        add_warning(
                            &mut report,
                            format!("条目已写入，但结构化迁移字段保存失败：{error}"),
                        );
                    } else {
                        report.imported += 1;
                    }
                }
                Err(error) => {
                    report.failed += 1;
                    add_warning(&mut report, format!("部分条目写入失败：{error}"));
                }
            }
        }
    })?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn imports_a_ditto_database_and_skips_duplicates() {
        let test_root =
            std::env::temp_dir().join(format!("tiez-ditto-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_root).expect("create test directory");
        let source_path = test_root.join("Ditto.db");
        let source = rusqlite::Connection::open(&source_path).expect("create source database");
        source
            .execute_batch(
                "CREATE TABLE Main (
                    lID INTEGER PRIMARY KEY,
                    lDate INTEGER,
                    mText TEXT,
                    lDontAutoDelete INTEGER,
                    bIsGroup INTEGER,
                    lParentID INTEGER,
                    stickyClipOrder REAL,
                    QuickPasteText TEXT,
                    lShortCut INTEGER,
                    globalShortCut INTEGER,
                    lastPasteDate INTEGER
                );
                CREATE TABLE Data (
                    lID INTEGER PRIMARY KEY,
                    lParentID INTEGER,
                    strClipBoardFormat TEXT,
                    ooData BLOB
                );",
            )
            .expect("create Ditto schema");
        source
            .execute(
                "INSERT INTO Main VALUES (
                    1, 0, 'Work', 0, 1, -1, -2147483647, '', 0, 0, 0
                )",
                [],
            )
            .expect("insert group");
        source
            .execute(
                "INSERT INTO Main VALUES (
                    2, 1700000000, 'Hello TieZ', 1, 0, 1, 9.5,
                    'hello-alias', 833, 1, 1700000100
                )",
                [],
            )
            .expect("insert clip");
        let text_blob = "Hello TieZ\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        source
            .execute(
                "INSERT INTO Data VALUES (1, 2, 'CF_UNICODETEXT', ?1)",
                params![text_blob],
            )
            .expect("insert text format");
        drop(source);

        let destination_path = test_root.join("clipboard.db");
        let destination = crate::database::init_db(&destination_path.to_string_lossy())
            .expect("create TieZ database");
        let destination = Arc::new(Mutex::new(destination));

        let first = import_clipboard_data(destination.clone(), &test_root, &source_path)
            .expect("first import");
        assert_eq!(first.source, "Ditto");
        assert_eq!(first.scanned, 1);
        assert_eq!(first.imported, 1);
        assert_eq!(first.duplicates, 0);

        let repo = SqliteClipboardRepository::new(destination.clone());
        let history = repo.get_history(10, 0, None).expect("read imported item");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello TieZ");
        assert_eq!(history[0].timestamp, 1_700_000_000_000);
        assert_eq!(history[0].created_at, 1_700_000_000_000);
        assert_eq!(history[0].sort_at, 1_700_000_000_000);
        assert_eq!(history[0].last_used_at, 1_700_000_100_000);
        assert!(history[0].is_pinned);
        assert_eq!(history[0].pinned_order, 9_500_000);
        assert_eq!(history[0].tags, vec!["Ditto · Work"]);
        assert_eq!(history[0].note, "hello-alias");
        assert_eq!(history[0].use_count, 0);
        drop(repo);

        let metadata: (String, i64, i64, String, i64) = destination
            .lock()
            .expect("lock destination")
            .query_row(
                "SELECT item_hotkey, item_hotkey_global, last_used_at,
                        move_to_group_hotkey, move_to_group_hotkey_global
                 FROM clipboard_history WHERE id = ?1",
                [history[0].id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read structured Ditto metadata");
        assert_eq!(
            metadata,
            (
                "Ctrl+Shift+A".to_string(),
                1,
                1_700_000_100_000,
                String::new(),
                0
            )
        );

        destination
            .lock()
            .expect("lock destination")
            .execute(
                "UPDATE clipboard_history SET
                    note = '',
                    item_hotkey = '',
                    item_hotkey_global = 0,
                    last_used_at = 0
                 WHERE id = ?1",
                [history[0].id],
            )
            .expect("clear metadata before duplicate merge");

        let second = import_clipboard_data(destination.clone(), &test_root, &source_path)
            .expect("second import");
        assert_eq!(second.imported, 0);
        assert_eq!(second.duplicates, 1);
        let merged: (String, String, i64, i64) = destination
            .lock()
            .expect("lock destination")
            .query_row(
                "SELECT note, item_hotkey, item_hotkey_global, last_used_at
                 FROM clipboard_history WHERE id = ?1",
                [history[0].id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read merged metadata");
        assert_eq!(
            merged,
            (
                "hello-alias".to_string(),
                "Ctrl+Shift+A".to_string(),
                1,
                1_700_000_100_000
            )
        );

        std::fs::remove_dir_all(&test_root).expect("remove test directory");
    }

    #[test]
    fn imports_a_maccy_database_with_distinct_creation_and_sort_times() {
        let test_root =
            std::env::temp_dir().join(format!("tiez-maccy-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_root).expect("create test directory");
        let source_path = test_root.join("Storage.sqlite");
        let source = rusqlite::Connection::open(&source_path).expect("create source database");
        source
            .execute_batch(
                "CREATE TABLE ZHISTORYITEM (
                    Z_PK INTEGER PRIMARY KEY,
                    ZFIRSTCOPIEDAT REAL,
                    ZLASTCOPIEDAT REAL,
                    ZNUMBEROFCOPIES INTEGER,
                    ZPIN TEXT,
                    ZTITLE TEXT,
                    ZAPPLICATION TEXT
                );
                CREATE TABLE ZHISTORYITEMCONTENT (
                    Z_PK INTEGER PRIMARY KEY,
                    ZITEM INTEGER,
                    ZTYPE TEXT,
                    ZVALUE BLOB
                );
                INSERT INTO ZHISTORYITEM VALUES (
                    1, 721692800.0, 721692900.0, 4, 'b',
                    'Hello from Maccy', 'com.apple.TextEdit'
                );
                INSERT INTO ZHISTORYITEM VALUES (
                    2, 721693000.0, 721693000.0, 1, NULL,
                    'https://maccy.app', 'com.apple.Safari'
                );",
            )
            .expect("create Maccy schema");
        source
            .execute(
                "INSERT INTO ZHISTORYITEMCONTENT VALUES (1, 1, 'public.utf8-plain-text', ?1)",
                [b"Hello from Maccy".as_slice()],
            )
            .expect("insert plain representation");
        source
            .execute(
                "INSERT INTO ZHISTORYITEMCONTENT VALUES (2, 1, 'public.html', ?1)",
                [b"<strong>Hello from Maccy</strong>".as_slice()],
            )
            .expect("insert html representation");
        source
            .execute(
                "INSERT INTO ZHISTORYITEMCONTENT VALUES (3, 1, 'public.rtf', ?1)",
                [br"{\rtf1\ansi Hello from Maccy}".as_slice()],
            )
            .expect("insert rtf representation");
        source
            .execute(
                "INSERT INTO ZHISTORYITEMCONTENT VALUES (4, 2, 'public.utf8-plain-text', ?1)",
                [b"https://maccy.app".as_slice()],
            )
            .expect("insert url");
        drop(source);

        let destination_path = test_root.join("clipboard.db");
        let destination = crate::database::init_db(&destination_path.to_string_lossy())
            .expect("create TieZ database");
        let destination = Arc::new(Mutex::new(destination));

        let report = import_clipboard_data(destination.clone(), &test_root, &source_path)
            .expect("import Maccy database");
        assert_eq!(report.source, "Maccy");
        assert_eq!(report.scanned, 2);
        assert_eq!(report.imported, 2);
        assert_eq!(report.unsupported, 0);
        assert_eq!(report.failed, 0);

        let repo = SqliteClipboardRepository::new(destination.clone());
        let history = repo.get_history(10, 0, None).expect("read imported items");
        let rich = history
            .iter()
            .find(|entry| entry.content == "Hello from Maccy")
            .expect("find rich Maccy item");
        assert_eq!(rich.content_type, "rich_text");
        assert_eq!(rich.source_app, "com.apple.TextEdit");
        assert_eq!(rich.created_at, 1_700_000_000_000);
        assert_eq!(rich.sort_at, 1_700_000_100_000);
        assert_eq!(rich.timestamp, rich.sort_at);
        assert_eq!(rich.last_used_at, 0);
        assert_eq!(rich.use_count, 4);
        assert!(rich.is_pinned);
        let (_, named_formats) = crate::services::clipboard::split_rich_html_and_named_formats(
            rich.html_content.as_deref().expect("stored rich html"),
        );
        assert!(named_formats
            .iter()
            .any(|format| format.name == "Rich Text Format"));

        let item_hotkey: (String, i64) = destination
            .lock()
            .expect("lock destination")
            .query_row(
                "SELECT item_hotkey, item_hotkey_global
                 FROM clipboard_history WHERE id = ?1",
                [rich.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read Maccy pin");
        assert_eq!(item_hotkey, ("B".to_string(), 0));

        let url = history
            .iter()
            .find(|entry| entry.content == "https://maccy.app")
            .expect("find url item");
        assert_eq!(url.content_type, "url");
        assert_eq!(url.source_app, "com.apple.Safari");

        drop(repo);
        drop(destination);
        std::fs::remove_dir_all(&test_root).expect("remove test directory");
    }
}
