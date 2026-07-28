use super::{ClipboardImporter, ImportCandidate, ImportedClip};
use crate::infrastructure::windows_api::win_clipboard::NamedClipboardFormat;
use crate::services::clipboard::{
    attach_rich_named_formats, derive_rich_text_content, detect_content_type, parse_cf_html,
    WINDOWS_STANDARD_FORMAT_PREFIX,
};
use base64::Engine;
use encoding_rs::{BIG5, EUC_KR, GBK, SHIFT_JIS, WINDOWS_1252};
use flate2::read::ZlibDecoder;
use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

const PAGE_SIZE: i64 = 250;
const MAX_FORMAT_BYTES: i64 = 64 * 1024 * 1024;
const DITTO_STICKY_SENTINEL: f64 = -2_147_483_647.0;
const IMPORT_NAMED_FORMAT_MAX_COUNT: usize = 32;
const IMPORT_NAMED_FORMAT_MAX_BYTES: usize = 8 * 1024 * 1024;
const IMPORT_NAMED_FORMAT_TOTAL_BYTES: usize = 24 * 1024 * 1024;

pub struct DittoImporter;

#[derive(Debug)]
struct DittoMainRow {
    id: i64,
    timestamp: i64,
    description: String,
    pinned: bool,
    pinned_order: i64,
    parent_id: i64,
    quick_paste: String,
    shortcut: i64,
    global_shortcut: bool,
    last_paste_date: i64,
    move_to_group_shortcut: i64,
    global_move_to_group_shortcut: bool,
}

#[derive(Debug)]
struct DittoFormat {
    name: String,
    bytes: Vec<u8>,
}

struct DittoSchema {
    main_columns: HashSet<String>,
    data_columns: HashSet<String>,
    exported: bool,
}

impl DittoSchema {
    fn read(conn: &Connection) -> Result<Option<Self>, String> {
        if !table_exists(conn, "Main")? || !table_exists(conn, "Data")? {
            return Ok(None);
        }
        let main_columns = table_columns(conn, "Main")?;
        let data_columns = table_columns(conn, "Data")?;
        let required_main = ["lid", "mtext"];
        let required_data = ["lparentid", "strclipboardformat", "oodata"];
        if !required_main
            .iter()
            .all(|name| main_columns.contains(*name))
            || !required_data
                .iter()
                .all(|name| data_columns.contains(*name))
        {
            return Ok(None);
        }
        let exported = main_columns.contains("lversion")
            && data_columns.contains("loriginalsize")
            && !main_columns.contains("ldate");
        Ok(Some(Self {
            main_columns,
            data_columns,
            exported,
        }))
    }

    fn main_expr(&self, column: &str, fallback: &str) -> String {
        if self.main_columns.contains(&column.to_ascii_lowercase()) {
            format!("COALESCE({column}, {fallback})")
        } else {
            fallback.to_string()
        }
    }
}

impl ClipboardImporter for DittoImporter {
    fn name(&self) -> &'static str {
        "Ditto"
    }

    fn can_import(&self, path: &Path) -> Result<bool, String> {
        let conn = open_read_only(path)?;
        Ok(DittoSchema::read(&conn)?.is_some())
    }

    fn visit_clips(
        &self,
        path: &Path,
        visitor: &mut dyn FnMut(ImportedClip),
    ) -> Result<(), String> {
        let conn = open_read_only(path)?;
        let schema =
            DittoSchema::read(&conn)?.ok_or_else(|| "文件不是受支持的 Ditto 数据库".to_string())?;
        let group_paths = load_group_paths(&conn, &schema)?;
        let max_id: i64 = conn
            .query_row("SELECT COALESCE(MAX(lID), 0) FROM Main", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("读取 Ditto 条目范围失败：{error}"))?;
        let fallback_time = source_modified_ms(path);

        let date_expr = schema.main_expr("lDate", "0");
        let pinned_expr = schema.main_expr("lDontAutoDelete", "0");
        let group_expr = schema.main_expr("bIsGroup", "0");
        let parent_expr = schema.main_expr("lParentID", "-1");
        let sticky_expr = schema.main_expr("stickyClipOrder", &DITTO_STICKY_SENTINEL.to_string());
        let sticky_group_expr =
            schema.main_expr("stickyClipGroupOrder", &DITTO_STICKY_SENTINEL.to_string());
        let quick_paste_expr = schema.main_expr("QuickPasteText", "''");
        let shortcut_expr = schema.main_expr("lShortCut", "0");
        let global_shortcut_expr = schema.main_expr("globalShortCut", "0");
        let last_paste_expr = schema.main_expr("lastPasteDate", "0");
        let move_shortcut_expr = schema.main_expr("MoveToGroupShortCut", "0");
        let global_move_shortcut_expr = schema.main_expr("GlobalMoveToGroupShortCut", "0");
        let main_sql = format!(
            "SELECT lID, {date_expr}, COALESCE(mText, ''), {pinned_expr}, \
             {parent_expr}, {sticky_expr}, {sticky_group_expr}, {quick_paste_expr}, \
             {shortcut_expr}, {global_shortcut_expr}, {last_paste_expr}, \
             {move_shortcut_expr}, {global_move_shortcut_expr} FROM Main \
             WHERE lID > ?1 AND {group_expr} = 0 ORDER BY lID ASC LIMIT ?2"
        );

        let mut cursor = 0_i64;
        loop {
            let page = {
                let mut statement = conn
                    .prepare(&main_sql)
                    .map_err(|error| format!("读取 Ditto 主表失败：{error}"))?;
                let rows = statement
                    .query_map([cursor, PAGE_SIZE], |row| {
                        let id: i64 = row.get(0)?;
                        let raw_timestamp: i64 = row.get(1).unwrap_or(0);
                        let sticky_order: f64 = row.get(5).unwrap_or(DITTO_STICKY_SENTINEL);
                        let sticky_group_order: f64 = row.get(6).unwrap_or(DITTO_STICKY_SENTINEL);
                        let parent_id = row.get(4).unwrap_or(-1);
                        let effective_sticky =
                            if parent_id > 0 && is_valid_sticky_order(sticky_group_order) {
                                sticky_group_order
                            } else {
                                sticky_order
                            };
                        let timestamp = normalize_timestamp(
                            raw_timestamp,
                            fallback_time.saturating_sub(max_id.saturating_sub(id)),
                        );
                        let never_auto_delete = row.get::<_, i64>(3).unwrap_or(0) != 0;
                        Ok(DittoMainRow {
                            id,
                            timestamp,
                            description: row.get(2).unwrap_or_default(),
                            pinned: never_auto_delete || is_valid_sticky_order(effective_sticky),
                            pinned_order: sticky_order_to_i64(effective_sticky, timestamp),
                            parent_id,
                            quick_paste: row.get(7).unwrap_or_default(),
                            shortcut: row.get(8).unwrap_or(0),
                            global_shortcut: row.get::<_, i64>(9).unwrap_or(0) != 0,
                            last_paste_date: row.get(10).unwrap_or(0),
                            move_to_group_shortcut: row.get(11).unwrap_or(0),
                            global_move_to_group_shortcut: row.get::<_, i64>(12).unwrap_or(0) != 0,
                        })
                    })
                    .map_err(|error| format!("遍历 Ditto 主表失败：{error}"))?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("解析 Ditto 主表失败：{error}"))?
            };
            if page.is_empty() {
                break;
            }

            for main in page {
                cursor = main.id;
                let (formats, format_warning) = load_formats(&conn, &schema, main.id)?;
                let tags = group_tag(main.parent_id, &group_paths);
                let mut clip = candidates_from_formats(&main, formats, tags);
                clip.warning = combine_warnings(clip.warning, format_warning);
                visitor(clip);
            }
        }

        Ok(())
    }
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| {
        format!(
            "无法只读打开 {}：{error}。如果 Ditto 开启了数据库加密，请先在 Ditto 中导出未加密数据。",
            path.display()
        )
    })?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    Ok(conn)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 COLLATE NOCASE)",
        [table],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn table_columns(conn: &Connection, table: &str) -> Result<HashSet<String>, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info(\"{table}\")"))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map(|name| name.to_ascii_lowercase()))
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|error| error.to_string())
}

fn source_modified_ms(path: &Path) -> i64 {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64
        })
}

fn normalize_timestamp(raw: i64, fallback: i64) -> i64 {
    if raw <= 0 {
        fallback
    } else if raw < 10_000_000_000 {
        raw.saturating_mul(1_000)
    } else {
        raw
    }
}

fn is_valid_sticky_order(order: f64) -> bool {
    order.is_finite() && (order - DITTO_STICKY_SENTINEL).abs() > f64::EPSILON
}

fn sticky_order_to_i64(order: f64, fallback: i64) -> i64 {
    if !is_valid_sticky_order(order) {
        return fallback;
    }
    let scaled = order * 1_000_000.0;
    scaled.clamp(i64::MIN as f64, i64::MAX as f64).round() as i64
}

fn combine_warnings(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) if first != second => Some(format!("{first}；{second}")),
        (Some(first), _) => Some(first),
        (_, Some(second)) => Some(second),
        _ => None,
    }
}

fn parse_cf_locale(bytes: &[u8]) -> Option<u32> {
    (bytes.len() >= 4).then(|| u32::from_le_bytes(bytes[0..4].try_into().unwrap()))
}

fn encoding_for_lcid(lcid: u32) -> &'static encoding_rs::Encoding {
    let primary_language = lcid & 0x03ff;
    match primary_language {
        0x04 => match lcid {
            0x0404 | 0x0c04 | 0x1404 => BIG5,
            _ => GBK,
        },
        0x11 => SHIFT_JIS,
        0x12 => EUC_KR,
        _ => WINDOWS_1252,
    }
}

fn collect_named_formats(formats: &[DittoFormat]) -> (Vec<NamedClipboardFormat>, Option<String>) {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut total_bytes = 0_usize;
    let mut skipped = 0_usize;
    let mut skipped_handle_formats = 0_usize;

    for format in formats {
        let name = format.name.trim();
        let lower = name.to_ascii_lowercase();
        let standard_id = standard_hglobal_format_id(&lower);
        let is_standard = lower.starts_with("cf_");
        let is_materialized_elsewhere = matches!(
            lower.as_str(),
            "html format" | "png" | "image/png" | "image/jpeg" | "image/gif" | "image/bmp"
        );
        if matches!(
            lower.as_str(),
            "cf_bitmap" | "cf_metafilepict" | "cf_palette" | "cf_enhmetafile"
        ) && !format.bytes.is_empty()
        {
            skipped_handle_formats += 1;
        }
        if name.is_empty()
            || format.bytes.is_empty()
            || (is_standard && standard_id.is_none())
            || is_materialized_elsewhere
            || !seen.insert(lower)
        {
            continue;
        }

        let would_exceed = result.len() >= IMPORT_NAMED_FORMAT_MAX_COUNT
            || format.bytes.len() > IMPORT_NAMED_FORMAT_MAX_BYTES
            || total_bytes.saturating_add(format.bytes.len()) > IMPORT_NAMED_FORMAT_TOTAL_BYTES;
        if would_exceed {
            skipped += 1;
            continue;
        }

        total_bytes += format.bytes.len();
        result.push(NamedClipboardFormat {
            name: standard_id
                .map(|id| format!("{WINDOWS_STANDARD_FORMAT_PREFIX}{id}"))
                .unwrap_or_else(|| name.to_string()),
            data: format.bytes.clone(),
        });
    }

    let mut warnings = Vec::new();
    if skipped > 0 {
        warnings.push(format!(
            "{skipped} 个过大或超出数量上限的 Ditto 应用自定义格式未附加（正文仍已导入）"
        ));
    }
    if skipped_handle_formats > 0 {
        warnings.push(format!(
            "{skipped_handle_formats} 个依赖 Windows GDI 句柄的旧式 Ditto 格式无法跨数据库恢复"
        ));
    }
    let warning = (!warnings.is_empty()).then(|| warnings.join("；"));
    (result, warning)
}

fn standard_hglobal_format_id(name: &str) -> Option<u32> {
    match name {
        "cf_sylk" => Some(4),
        "cf_dif" => Some(5),
        "cf_tiff" => Some(6),
        "cf_pendata" => Some(10),
        "cf_riff" => Some(11),
        "cf_wave" => Some(12),
        _ => None,
    }
}

fn html_from_plain_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 32);
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(ch),
        }
    }
    format!("<pre style=\"white-space: pre-wrap\">{escaped}</pre>")
}

fn ditto_note(main: &DittoMainRow) -> String {
    main.quick_paste.trim().to_string()
}

fn ditto_hotkey(value: i64) -> String {
    (value > 0)
        .then(|| format_ditto_hotkey(value))
        .unwrap_or_default()
}

fn ditto_last_used_at(main: &DittoMainRow) -> i64 {
    normalize_timestamp(main.last_paste_date, 0)
}

fn format_ditto_hotkey(value: i64) -> String {
    let packed = value as u16;
    let key = (packed & 0xff) as u8;
    let modifiers = (packed >> 8) as u8;
    let mut parts = Vec::new();
    if modifiers & 0x02 != 0 {
        parts.push("Ctrl".to_string());
    }
    if modifiers & 0x01 != 0 {
        parts.push("Shift".to_string());
    }
    if modifiers & 0x04 != 0 {
        parts.push("Alt".to_string());
    }
    parts.push(match key {
        0x30..=0x39 | 0x41..=0x5a => (key as char).to_string(),
        0x70..=0x87 => format!("F{}", key - 0x6f),
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0d => "Enter".to_string(),
        0x1b => "Esc".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2e => "Delete".to_string(),
        _ => format!("VK_{key:02X}"),
    });
    parts.join("+")
}

fn load_group_paths(
    conn: &Connection,
    schema: &DittoSchema,
) -> Result<HashMap<i64, String>, String> {
    if !schema.main_columns.contains("bisgroup") {
        return Ok(HashMap::new());
    }
    let parent_expr = schema.main_expr("lParentID", "-1");
    let sql =
        format!("SELECT lID, COALESCE(mText, ''), {parent_expr} FROM Main WHERE bIsGroup <> 0");
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, i64>(2).unwrap_or(-1),
            ))
        })
        .map_err(|error| error.to_string())?;
    let groups = rows
        .map(|row| row.map(|(id, name, parent)| (id, (name, parent))))
        .collect::<Result<HashMap<i64, (String, i64)>, _>>()
        .map_err(|error| error.to_string())?;
    let mut paths = HashMap::new();
    for id in groups.keys().copied() {
        if let Some(path) = resolve_group_path(id, &groups) {
            paths.insert(id, path);
        }
    }
    Ok(paths)
}

fn resolve_group_path(id: i64, groups: &HashMap<i64, (String, i64)>) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = id;
    let mut seen = HashSet::new();
    while current > 0 && seen.insert(current) {
        let (name, parent) = groups.get(&current)?;
        let clean = name.trim();
        if !clean.is_empty() {
            parts.push(clean.to_string());
        }
        current = *parent;
    }
    parts.reverse();
    (!parts.is_empty()).then(|| parts.join(" / "))
}

fn group_tag(parent_id: i64, group_paths: &HashMap<i64, String>) -> Vec<String> {
    group_paths
        .get(&parent_id)
        .map(|path| vec![format!("Ditto · {path}")])
        .unwrap_or_default()
}

fn load_formats(
    conn: &Connection,
    schema: &DittoSchema,
    parent_id: i64,
) -> Result<(Vec<DittoFormat>, Option<String>), String> {
    let original_size_expr = if schema.data_columns.contains("loriginalsize") {
        "COALESCE(lOriginalSize, 0)"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT COALESCE(strClipBoardFormat, ''), \
         CASE WHEN length(ooData) <= ?2 THEN ooData ELSE NULL END, \
         COALESCE(length(ooData), 0), {original_size_expr} \
         FROM Data WHERE lParentID = ?1 ORDER BY lID DESC"
    );
    let mut statement = conn
        .prepare(&sql)
        .map_err(|error| format!("读取 Ditto Data 表失败：{error}"))?;
    let mut rows = statement
        .query([parent_id, MAX_FORMAT_BYTES])
        .map_err(|error| format!("查询 Ditto Data 表失败：{error}"))?;
    let mut formats = Vec::new();
    let mut skipped_large = false;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let name: String = row.get(0).unwrap_or_default();
        let stored_len: i64 = row.get(2).unwrap_or(0);
        let original_size: i64 = row.get(3).unwrap_or(0);
        let Some(mut bytes) = row.get::<_, Option<Vec<u8>>>(1).unwrap_or(None) else {
            if stored_len > MAX_FORMAT_BYTES {
                skipped_large = true;
            }
            continue;
        };
        if schema.exported {
            match decompress_export_blob(&bytes, original_size) {
                Ok(decoded) => bytes = decoded,
                Err(_) => continue,
            }
        }
        formats.push(DittoFormat { name, bytes });
    }
    let warning = skipped_large.then(|| "已跳过超过 64 MB 的单个 Ditto 数据格式".to_string());
    Ok((formats, warning))
}

fn decompress_export_blob(bytes: &[u8], expected_size: i64) -> Result<Vec<u8>, String> {
    if expected_size <= 0 || expected_size > MAX_FORMAT_BYTES {
        return Err("Ditto 导出条目大小无效".to_string());
    }
    let decoder = ZlibDecoder::new(bytes);
    let mut output = Vec::with_capacity(expected_size as usize);
    decoder
        .take((MAX_FORMAT_BYTES + 1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| error.to_string())?;
    if output.len() as i64 > MAX_FORMAT_BYTES {
        return Err("Ditto 导出条目解压后过大".to_string());
    }
    Ok(output)
}

fn candidates_from_formats(
    main: &DittoMainRow,
    formats: Vec<DittoFormat>,
    tags: Vec<String>,
) -> ImportedClip {
    let mut by_name = HashMap::<String, &Vec<u8>>::new();
    for format in &formats {
        by_name
            .entry(format.name.trim().to_ascii_lowercase())
            .or_insert(&format.bytes);
    }
    let locale = by_name
        .get("cf_locale")
        .and_then(|bytes| parse_cf_locale(bytes));
    let (named_formats, named_warning) = collect_named_formats(&formats);
    let note = ditto_note(main);
    let use_count = 0;

    if let Some(drop_bytes) = by_name.get("cf_hdrop") {
        let paths = parse_hdrop(drop_bytes, locale);
        if !paths.is_empty() {
            let candidates = paths
                .into_iter()
                .enumerate()
                .map(|(index, path)| ImportCandidate {
                    content_type: file_content_type(&path),
                    content: path,
                    html_content: None,
                    created_at: main.timestamp,
                    sort_at: main.timestamp.saturating_add(index as i64),
                    source_app: "Ditto".to_string(),
                    is_pinned: main.pinned,
                    pinned_order: if main.pinned { main.pinned_order } else { 0 },
                    tags: tags.clone(),
                    note: note.clone(),
                    use_count,
                    item_hotkey: ditto_hotkey(main.shortcut),
                    item_hotkey_global: main.global_shortcut,
                    last_used_at: ditto_last_used_at(main),
                    move_to_group_hotkey: ditto_hotkey(main.move_to_group_shortcut),
                    move_to_group_hotkey_global: main.global_move_to_group_shortcut,
                    is_external: true,
                })
                .collect();
            return ImportedClip {
                candidates,
                warning: named_warning,
            };
        }
    }

    let unicode_text = by_name
        .get("cf_unicodetext")
        .and_then(|bytes| decode_utf16_le(bytes));
    let ansi_text = by_name
        .get("cf_text")
        .or_else(|| by_name.get("cf_oemtext"))
        .and_then(|bytes| decode_legacy_text(bytes, locale));
    let utf8_text = by_name
        .get("utf8_string")
        .or_else(|| by_name.get("text/plain;charset=utf-8"))
        .and_then(|bytes| decode_legacy_text(bytes, Some(0)));
    let plain_text = unicode_text
        .or(utf8_text)
        .or(ansi_text)
        .filter(|text| !text.is_empty());

    if let Some(html_bytes) = by_name.get("html format") {
        if let Some(html) = parse_cf_html(html_bytes).filter(|html| !html.trim().is_empty()) {
            let content = plain_text
                .clone()
                .filter(|text| !text.trim().is_empty())
                .unwrap_or_else(|| derive_rich_text_content("", Some(&html)));
            if !content.is_empty() {
                let html = attach_rich_named_formats(&html, &named_formats);
                return ImportedClip {
                    candidates: vec![candidate(
                        main,
                        "rich_text",
                        content,
                        Some(html),
                        tags,
                        note,
                        use_count,
                        false,
                    )],
                    warning: named_warning,
                };
            }
        }
    }

    if let Some(text) = plain_text {
        if !named_formats.is_empty() {
            let html = attach_rich_named_formats(&html_from_plain_text(&text), &named_formats);
            return ImportedClip {
                candidates: vec![candidate(
                    main,
                    "rich_text",
                    text,
                    Some(html),
                    tags,
                    note,
                    use_count,
                    false,
                )],
                warning: named_warning,
            };
        }
        let content_type = detect_content_type(&text);
        return ImportedClip {
            candidates: vec![candidate(
                main,
                &content_type,
                text,
                None,
                tags,
                note,
                use_count,
                false,
            )],
            warning: named_warning,
        };
    }

    if !named_formats.is_empty() {
        let mut content = main.description.trim().to_string();
        if content.is_empty() {
            content = named_formats
                .iter()
                .find(|format| format.name.eq_ignore_ascii_case("Rich Text Format"))
                .map(|format| plain_text_from_rtf(&format.data))
                .unwrap_or_default();
        }
        if content.is_empty() {
            let format_names = formats
                .iter()
                .map(|format| format.name.trim())
                .filter(|name| !name.is_empty())
                .take(4)
                .collect::<Vec<_>>()
                .join(", ");
            content = if format_names.is_empty() {
                "Ditto 原始剪贴板数据".to_string()
            } else {
                format!("Ditto 原始剪贴板数据（{format_names}）")
            };
        }
        if !content.is_empty() {
            let html = attach_rich_named_formats(&html_from_plain_text(&content), &named_formats);
            return ImportedClip {
                candidates: vec![candidate(
                    main,
                    "rich_text",
                    content,
                    Some(html),
                    tags,
                    note,
                    use_count,
                    false,
                )],
                warning: named_warning,
            };
        }
    }

    for name in ["png", "image/png", "cf_dibv5", "cf_dib"] {
        if let Some(bytes) = by_name.get(name) {
            if let Some(data_url) = image_data_url(name, bytes) {
                return ImportedClip {
                    candidates: vec![candidate(
                        main, "image", data_url, None, tags, note, use_count, false,
                    )],
                    warning: named_warning,
                };
            }
        }
    }

    let warning = if !main.description.trim().is_empty() {
        Some(format!(
            "不支持的 Ditto 条目格式：{}",
            main.description.chars().take(40).collect::<String>()
        ))
    } else {
        named_warning
    };
    ImportedClip {
        candidates: Vec::new(),
        warning,
    }
}

fn candidate(
    main: &DittoMainRow,
    content_type: &str,
    content: String,
    html_content: Option<String>,
    tags: Vec<String>,
    note: String,
    use_count: i32,
    is_external: bool,
) -> ImportCandidate {
    ImportCandidate {
        content_type: content_type.to_string(),
        content,
        html_content,
        created_at: main.timestamp,
        sort_at: main.timestamp,
        source_app: "Ditto".to_string(),
        is_pinned: main.pinned,
        pinned_order: if main.pinned { main.pinned_order } else { 0 },
        tags,
        note,
        use_count,
        item_hotkey: ditto_hotkey(main.shortcut),
        item_hotkey_global: main.global_shortcut,
        last_used_at: ditto_last_used_at(main),
        move_to_group_hotkey: ditto_hotkey(main.move_to_group_shortcut),
        move_to_group_hotkey_global: main.global_move_to_group_shortcut,
        is_external,
    }
}

fn decode_utf16_le(bytes: &[u8]) -> Option<String> {
    let mut units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    if units.first() == Some(&0xfeff) {
        units.remove(0);
    }
    while units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16(&units).ok()
}

fn decode_legacy_text(bytes: &[u8], locale: Option<u32>) -> Option<String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if end == 0 {
        return None;
    }
    let raw = &bytes[..end];
    if let Ok(text) = std::str::from_utf8(raw) {
        return Some(text.to_string());
    }
    let encoding = encoding_for_lcid(locale.unwrap_or_default());
    let (text, _, _) = encoding.decode(raw);
    Some(text.into_owned())
}

#[derive(Clone, Copy)]
struct RtfState {
    skip_destination: bool,
    unicode_fallback_len: usize,
}

pub(super) fn plain_text_from_rtf(bytes: &[u8]) -> String {
    let mut output = String::new();
    let mut states = vec![RtfState {
        skip_destination: false,
        unicode_fallback_len: 1,
    }];
    let mut index = 0_usize;
    let mut fallback_remaining = 0_usize;
    let mut encoding = WINDOWS_1252;

    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'{' => {
                let state = *states.last().unwrap();
                states.push(state);
                index += 1;
            }
            b'}' => {
                if states.len() > 1 {
                    states.pop();
                }
                index += 1;
            }
            b'\\' => {
                index += 1;
                if index >= bytes.len() {
                    break;
                }
                let symbol = bytes[index];
                if matches!(symbol, b'\\' | b'{' | b'}') {
                    if fallback_remaining > 0 {
                        fallback_remaining -= 1;
                    } else if !states.last().unwrap().skip_destination {
                        output.push(symbol as char);
                    }
                    index += 1;
                    continue;
                }
                if symbol == b'\'' && index + 2 < bytes.len() {
                    if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                        if let Ok(value) = u8::from_str_radix(hex, 16) {
                            if fallback_remaining > 0 {
                                fallback_remaining -= 1;
                            } else if !states.last().unwrap().skip_destination {
                                let raw = [value];
                                let (text, _, _) = encoding.decode(&raw);
                                output.push_str(&text);
                            }
                        }
                    }
                    index += 3;
                    continue;
                }
                if symbol == b'*' {
                    states.last_mut().unwrap().skip_destination = true;
                    index += 1;
                    continue;
                }
                if !symbol.is_ascii_alphabetic() {
                    if !states.last().unwrap().skip_destination {
                        match symbol {
                            b'~' => output.push('\u{00a0}'),
                            b'_' => output.push('\u{2011}'),
                            b'-' => {}
                            _ => {}
                        }
                    }
                    index += 1;
                    continue;
                }

                let word_start = index;
                while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                    index += 1;
                }
                let word = std::str::from_utf8(&bytes[word_start..index]).unwrap_or_default();
                let mut negative = false;
                if index < bytes.len() && bytes[index] == b'-' {
                    negative = true;
                    index += 1;
                }
                let number_start = index;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
                let number = if number_start < index {
                    std::str::from_utf8(&bytes[number_start..index])
                        .ok()
                        .and_then(|value| value.parse::<i32>().ok())
                        .map(|value| if negative { -value } else { value })
                } else {
                    None
                };
                if index < bytes.len() && bytes[index] == b' ' {
                    index += 1;
                }

                let destination = matches!(
                    word,
                    "fonttbl"
                        | "colortbl"
                        | "stylesheet"
                        | "info"
                        | "pict"
                        | "object"
                        | "filetbl"
                        | "listtable"
                        | "listoverridetable"
                        | "generator"
                        | "datastore"
                        | "themedata"
                        | "colorschememapping"
                        | "xmlnstbl"
                        | "header"
                        | "footer"
                        | "headerl"
                        | "headerr"
                        | "footerl"
                        | "footerr"
                );
                if destination {
                    states.last_mut().unwrap().skip_destination = true;
                    continue;
                }
                if word == "uc" {
                    if let Some(value) = number {
                        states.last_mut().unwrap().unicode_fallback_len = value.max(0) as usize;
                    }
                    continue;
                }
                if word == "ansicpg" {
                    if let Some(code_page) = number {
                        encoding = match code_page {
                            936 => GBK,
                            950 => BIG5,
                            932 => SHIFT_JIS,
                            949 => EUC_KR,
                            _ => WINDOWS_1252,
                        };
                    }
                    continue;
                }
                if states.last().unwrap().skip_destination {
                    continue;
                }
                match word {
                    "par" | "line" => output.push('\n'),
                    "tab" => output.push('\t'),
                    "emdash" => output.push('\u{2014}'),
                    "endash" => output.push('\u{2013}'),
                    "lquote" => output.push('\u{2018}'),
                    "rquote" => output.push('\u{2019}'),
                    "ldblquote" => output.push('\u{201c}'),
                    "rdblquote" => output.push('\u{201d}'),
                    "bullet" => output.push('\u{2022}'),
                    "u" => {
                        if let Some(value) = number {
                            let unit = value as i16 as u16;
                            if let Some(ch) = char::from_u32(unit as u32) {
                                output.push(ch);
                            }
                            fallback_remaining = states.last().unwrap().unicode_fallback_len;
                        }
                    }
                    _ => {}
                }
            }
            b'\r' | b'\n' | 0 => {
                index += 1;
            }
            _ => {
                if fallback_remaining > 0 {
                    fallback_remaining -= 1;
                } else if !states.last().unwrap().skip_destination {
                    if byte.is_ascii() {
                        output.push(byte as char);
                    } else {
                        let raw = [byte];
                        let (text, _, _) = encoding.decode(&raw);
                        output.push_str(&text);
                    }
                }
                index += 1;
            }
        }
    }

    output.trim().to_string()
}

fn parse_hdrop(bytes: &[u8], locale: Option<u32>) -> Vec<String> {
    if bytes.len() < 20 {
        return Vec::new();
    }
    let offset = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let wide = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) != 0;
    if offset >= bytes.len() {
        return Vec::new();
    }
    if wide {
        let units = bytes[offset..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        units
            .split(|unit| *unit == 0)
            .take_while(|part| !part.is_empty())
            .filter_map(|part| String::from_utf16(part).ok())
            .filter(|path| !path.trim().is_empty())
            .collect()
    } else {
        bytes[offset..]
            .split(|byte| *byte == 0)
            .take_while(|part| !part.is_empty())
            .filter_map(|part| decode_legacy_text(part, locale))
            .filter(|path| !path.trim().is_empty())
            .collect()
    }
}

fn file_content_type(path: &str) -> String {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        extension.as_str(),
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v"
    ) {
        "video".to_string()
    } else {
        "file".to_string()
    }
}

fn image_data_url(format_name: &str, bytes: &[u8]) -> Option<String> {
    let png = if matches!(format_name, "png" | "image/png") {
        image::load_from_memory(bytes).ok()?;
        bytes.to_vec()
    } else {
        let bmp = bmp_from_dib(bytes)?;
        let image = image::load_from_memory(&bmp).ok()?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        image.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
        cursor.into_inner()
    };
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

fn bmp_from_dib(dib: &[u8]) -> Option<Vec<u8>> {
    if dib.len() < 12 {
        return None;
    }
    let header_size = u32::from_le_bytes(dib[0..4].try_into().ok()?) as usize;
    if header_size < 12 || header_size > dib.len() {
        return None;
    }
    let (bit_count, compression, colors_used, palette_entry_size) = if header_size == 12 {
        (
            u16::from_le_bytes(dib[10..12].try_into().ok()?),
            0_u32,
            0_u32,
            3_usize,
        )
    } else if dib.len() >= 40 {
        (
            u16::from_le_bytes(dib[14..16].try_into().ok()?),
            u32::from_le_bytes(dib[16..20].try_into().ok()?),
            u32::from_le_bytes(dib[32..36].try_into().ok()?),
            4_usize,
        )
    } else {
        return None;
    };
    let palette_colors = if colors_used > 0 {
        colors_used as usize
    } else if bit_count <= 8 {
        1_usize.checked_shl(bit_count as u32)?
    } else {
        0
    };
    let masks_size = if header_size == 40 && compression == 3 {
        12
    } else if header_size == 40 && compression == 6 {
        16
    } else {
        0
    };
    let pixel_offset = 14_usize
        .checked_add(header_size)?
        .checked_add(masks_size)?
        .checked_add(palette_colors.checked_mul(palette_entry_size)?)?;
    if pixel_offset.saturating_sub(14) > dib.len() {
        return None;
    }
    let file_size = 14_usize.checked_add(dib.len())?;
    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&(pixel_offset as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    Some(bmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    #[test]
    fn decodes_unicode_text_and_trailing_nulls() {
        let bytes = "你好 TieZ\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(decode_utf16_le(&bytes).as_deref(), Some("你好 TieZ"));
    }

    #[test]
    fn parses_wide_hdrop_paths() {
        let mut bytes = vec![0_u8; 20];
        bytes[0..4].copy_from_slice(&20_u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&1_u32.to_le_bytes());
        for unit in "C:\\one.txt\0D:\\two.png\0\0".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(
            parse_hdrop(&bytes, None),
            vec!["C:\\one.txt".to_string(), "D:\\two.png".to_string()]
        );
    }

    #[test]
    fn decodes_simplified_chinese_cf_text_from_cf_locale() {
        let locale = parse_cf_locale(&0x0804_u32.to_le_bytes());
        assert_eq!(
            decode_legacy_text(&[0xc4, 0xe3, 0xba, 0xc3, 0], locale).as_deref(),
            Some("你好")
        );
    }

    #[test]
    fn extracts_unicode_text_from_rtf() {
        let rtf = br"{\rtf1\ansi\uc1 Hello \u20320?\u22909?\par TieZ}";
        assert_eq!(plain_text_from_rtf(rtf), "Hello 你好\nTieZ");
    }

    #[test]
    fn preserves_rtf_custom_formats_and_ditto_metadata() {
        let main = DittoMainRow {
            id: 7,
            timestamp: 1_700_000_000_000,
            description: "Hello".to_string(),
            pinned: true,
            pinned_order: 42_000_000,
            parent_id: -1,
            quick_paste: "greeting".to_string(),
            shortcut: 0x0341,
            global_shortcut: true,
            last_paste_date: 1_700_000_100,
            move_to_group_shortcut: 0,
            global_move_to_group_shortcut: false,
        };
        let formats = vec![
            DittoFormat {
                name: "CF_UNICODETEXT".to_string(),
                bytes: "Hello\0"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect(),
            },
            DittoFormat {
                name: "Rich Text Format".to_string(),
                bytes: br"{\rtf1 Hello}".to_vec(),
            },
            DittoFormat {
                name: "Biff12".to_string(),
                bytes: vec![1, 2, 3],
            },
        ];
        let imported = candidates_from_formats(&main, formats, vec![]);
        let candidate = imported.candidates.first().expect("one rich candidate");
        assert_eq!(candidate.content_type, "rich_text");
        assert_eq!(candidate.pinned_order, 42_000_000);
        assert_eq!(candidate.use_count, 0);
        assert_eq!(candidate.note, "greeting");
        assert_eq!(candidate.item_hotkey, "Ctrl+Shift+A");
        assert!(candidate.item_hotkey_global);
        assert_eq!(candidate.last_used_at, 1_700_000_100_000);
        let (_, named) = crate::services::clipboard::split_rich_html_and_named_formats(
            candidate.html_content.as_deref().expect("stored rich html"),
        );
        assert!(named.iter().any(|format| format.name == "Rich Text Format"));
        assert!(named.iter().any(|format| format.name == "Biff12"));
    }

    #[test]
    fn preserves_hglobal_backed_standard_windows_formats() {
        let main = DittoMainRow {
            id: 8,
            timestamp: 1_700_000_000_000,
            description: String::new(),
            pinned: false,
            pinned_order: 0,
            parent_id: -1,
            quick_paste: String::new(),
            shortcut: 0,
            global_shortcut: false,
            last_paste_date: 0,
            move_to_group_shortcut: 0,
            global_move_to_group_shortcut: false,
        };
        let imported = candidates_from_formats(
            &main,
            vec![DittoFormat {
                name: "CF_WAVE".to_string(),
                bytes: b"RIFF....WAVE".to_vec(),
            }],
            vec![],
        );
        let candidate = imported.candidates.first().expect("raw format candidate");
        assert!(candidate.content.contains("CF_WAVE"));
        let (_, named) = crate::services::clipboard::split_rich_html_and_named_formats(
            candidate.html_content.as_deref().expect("stored marker"),
        );
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].name, format!("{WINDOWS_STANDARD_FORMAT_PREFIX}12"));
    }

    #[test]
    fn keeps_ditto_sticky_sort_order() {
        assert!(sticky_order_to_i64(12.5, 1) > sticky_order_to_i64(11.0, 2));
        assert_eq!(sticky_order_to_i64(DITTO_STICKY_SENTINEL, 99), 99);
    }

    #[test]
    fn wraps_dib_as_bmp() {
        let mut dib = Vec::new();
        dib.extend_from_slice(&40_u32.to_le_bytes());
        dib.extend_from_slice(&1_i32.to_le_bytes());
        dib.extend_from_slice(&1_i32.to_le_bytes());
        dib.extend_from_slice(&1_u16.to_le_bytes());
        dib.extend_from_slice(&24_u16.to_le_bytes());
        dib.extend_from_slice(&0_u32.to_le_bytes());
        dib.extend_from_slice(&4_u32.to_le_bytes());
        dib.extend_from_slice(&[0; 16]);
        dib.extend_from_slice(&[0, 0, 255, 0]);
        let bmp = bmp_from_dib(&dib).expect("valid bmp");
        let image = image::load_from_memory(&bmp).expect("decodable bmp");
        assert_eq!((image.width(), image.height()), (1, 1));
    }

    #[test]
    fn decompresses_ditto_export_format() {
        let original = b"Ditto export payload";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).expect("compress fixture");
        let compressed = encoder.finish().expect("finish fixture");
        assert_eq!(
            decompress_export_blob(&compressed, original.len() as i64).expect("decompress"),
            original
        );
    }
}
