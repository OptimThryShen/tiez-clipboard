use super::{ClipboardImporter, ImportCandidate, ImportedClip};
use crate::infrastructure::windows_api::win_clipboard::NamedClipboardFormat;
use crate::services::clipboard::{
    attach_rich_named_formats, derive_rich_text_content, detect_content_type,
};
use base64::Engine;
use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::UNIX_EPOCH;

#[cfg(target_os = "macos")]
use objc2::rc::autoreleasepool;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSBitmapImageRepPropertyKey};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSData, NSDictionary};

const PAGE_SIZE: i64 = 250;
const MAX_CONTENT_BYTES: i64 = 64 * 1024 * 1024;
const APPLE_REFERENCE_DATE_UNIX_SECONDS: f64 = 978_307_200.0;

const TYPE_STRING: &[&str] = &["public.utf8-plain-text", "NSStringPboardType"];
const TYPE_HTML: &[&str] = &["public.html", "NSHTMLPboardType"];
const TYPE_RTF: &[&str] = &["public.rtf", "NSRTFPboardType"];
const TYPE_FILE_URL: &[&str] = &["public.file-url"];
const TYPE_PNG: &[&str] = &["public.png"];
const TYPE_JPEG: &[&str] = &["public.jpeg", "public.jpg"];
const TYPE_TIFF: &[&str] = &["public.tiff", "NSTIFFPboardType"];
const TYPE_HEIC: &[&str] = &["public.heic", "public.heif"];

pub struct MaccyImporter;

#[derive(Debug)]
struct MaccySchema {
    item_columns: HashSet<String>,
    content_columns: HashSet<String>,
}

#[derive(Debug)]
struct MaccyItem {
    id: i64,
    created_at: i64,
    sort_at: i64,
    use_count: i32,
    pin: Option<String>,
    title: String,
    application: String,
}

#[derive(Debug)]
struct MaccyContent {
    data_type: String,
    bytes: Vec<u8>,
}

impl MaccySchema {
    fn read(conn: &Connection) -> Result<Option<Self>, String> {
        if !table_exists(conn, "ZHISTORYITEM")? || !table_exists(conn, "ZHISTORYITEMCONTENT")? {
            return Ok(None);
        }
        let item_columns = table_columns(conn, "ZHISTORYITEM")?;
        let content_columns = table_columns(conn, "ZHISTORYITEMCONTENT")?;
        let has_item_date = ["zfirstcopiedat", "zlastcopiedat", "zcopiedat", "ztimestamp"]
            .iter()
            .any(|column| item_columns.contains(*column));
        let valid = item_columns.contains("z_pk")
            && has_item_date
            && ["zitem", "ztype", "zvalue"]
                .iter()
                .all(|column| content_columns.contains(*column));
        Ok(valid.then_some(Self {
            item_columns,
            content_columns,
        }))
    }

    fn item_expr(&self, candidates: &[&str], fallback: &str) -> String {
        candidates
            .iter()
            .find(|column| self.item_columns.contains(&column.to_ascii_lowercase()))
            .map(|column| format!("COALESCE({column}, {fallback})"))
            .unwrap_or_else(|| fallback.to_string())
    }
}

impl ClipboardImporter for MaccyImporter {
    fn name(&self) -> &'static str {
        "Maccy"
    }

    fn can_import(&self, path: &Path) -> Result<bool, String> {
        let conn = open_read_only(path)?;
        Ok(MaccySchema::read(&conn)?.is_some())
    }

    fn visit_clips(
        &self,
        path: &Path,
        visitor: &mut dyn FnMut(ImportedClip),
    ) -> Result<(), String> {
        let conn = open_read_only(path)?;
        let schema =
            MaccySchema::read(&conn)?.ok_or_else(|| "文件不是受支持的 Maccy 数据库".to_string())?;
        conn.execute_batch("BEGIN DEFERRED TRANSACTION")
            .map_err(|error| format!("无法创建 Maccy 只读快照：{error}"))?;
        let result = visit_snapshot(&conn, &schema, path, visitor);
        let _ = conn.execute_batch("ROLLBACK");
        result
    }
}

fn visit_snapshot(
    conn: &Connection,
    schema: &MaccySchema,
    path: &Path,
    visitor: &mut dyn FnMut(ImportedClip),
) -> Result<(), String> {
    let fallback_time = source_modified_ms(path);
    let first_expr = schema.item_expr(
        &["ZFIRSTCOPIEDAT", "ZCOPIEDAT", "ZTIMESTAMP", "ZLASTCOPIEDAT"],
        "0",
    );
    let last_expr = schema.item_expr(
        &["ZLASTCOPIEDAT", "ZCOPIEDAT", "ZTIMESTAMP", "ZFIRSTCOPIEDAT"],
        "0",
    );
    let copies_expr = schema.item_expr(&["ZNUMBEROFCOPIES"], "1");
    let pin_expr = schema.item_expr(&["ZPIN"], "NULL");
    let title_expr = schema.item_expr(&["ZTITLE"], "''");
    let application_expr = schema.item_expr(&["ZAPPLICATION"], "''");
    let item_sql = format!(
        "SELECT Z_PK, {first_expr}, {last_expr}, {copies_expr}, {pin_expr}, \
         {title_expr}, {application_expr}
         FROM ZHISTORYITEM WHERE Z_PK > ?1 ORDER BY Z_PK ASC LIMIT ?2"
    );
    let max_id = conn
        .query_row(
            "SELECT COALESCE(MAX(Z_PK), 0) FROM ZHISTORYITEM",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("读取 Maccy 条目范围失败：{error}"))?;

    let mut cursor = 0_i64;
    loop {
        let items = {
            let mut statement = conn
                .prepare(&item_sql)
                .map_err(|error| format!("读取 Maccy 主表失败：{error}"))?;
            let rows = statement
                .query_map([cursor, PAGE_SIZE], |row| {
                    let id: i64 = row.get(0)?;
                    let raw_first = numeric_value(row.get_ref(1)?);
                    let raw_last = numeric_value(row.get_ref(2)?);
                    let mut created_at = core_data_date_to_unix_ms(raw_first);
                    let mut sort_at = core_data_date_to_unix_ms(raw_last);
                    if created_at <= 0 {
                        created_at = sort_at;
                    }
                    if sort_at <= 0 {
                        sort_at = created_at;
                    }
                    if created_at <= 0 {
                        created_at = fallback_time.saturating_sub(max_id.saturating_sub(id));
                    }
                    if sort_at <= 0 {
                        sort_at = created_at;
                    }
                    let copies = row.get::<_, i64>(3).unwrap_or(1).clamp(0, i32::MAX as i64);
                    Ok(MaccyItem {
                        id,
                        created_at,
                        sort_at,
                        use_count: copies as i32,
                        pin: row.get(4).unwrap_or(None),
                        title: row.get(5).unwrap_or_default(),
                        application: row.get(6).unwrap_or_default(),
                    })
                })
                .map_err(|error| format!("遍历 Maccy 主表失败：{error}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("解析 Maccy 主表失败：{error}"))?
        };
        if items.is_empty() {
            break;
        }

        for item in items {
            cursor = item.id;
            let (contents, warning) = load_contents(conn, schema, item.id)?;
            let mut clip = candidates_from_contents(&item, contents);
            clip.warning = combine_warnings(clip.warning, warning);
            visitor(clip);
        }
    }
    Ok(())
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("无法只读打开 {}：{error}", path.display()))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    Ok(conn)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type = 'table' AND name = ?1 COLLATE NOCASE
        )",
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
        .unwrap_or_else(now_ms)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn numeric_value(value: rusqlite::types::ValueRef<'_>) -> f64 {
    match value {
        rusqlite::types::ValueRef::Integer(value) => value as f64,
        rusqlite::types::ValueRef::Real(value) => value,
        rusqlite::types::ValueRef::Text(value) => std::str::from_utf8(value)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

fn core_data_date_to_unix_ms(value: f64) -> i64 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    ((value + APPLE_REFERENCE_DATE_UNIX_SECONDS) * 1_000.0)
        .round()
        .clamp(0.0, i64::MAX as f64) as i64
}

fn load_contents(
    conn: &Connection,
    schema: &MaccySchema,
    item_id: i64,
) -> Result<(Vec<MaccyContent>, Option<String>), String> {
    debug_assert!(schema.content_columns.contains("zvalue"));
    let mut statement = conn
        .prepare(
            "SELECT COALESCE(ZTYPE, ''),
                    CASE WHEN length(ZVALUE) <= ?2 THEN ZVALUE ELSE NULL END,
                    COALESCE(length(ZVALUE), 0)
             FROM ZHISTORYITEMCONTENT
             WHERE ZITEM = ?1 ORDER BY Z_PK ASC",
        )
        .map_err(|error| format!("读取 Maccy 内容表失败：{error}"))?;
    let mut rows = statement
        .query([item_id, MAX_CONTENT_BYTES])
        .map_err(|error| format!("查询 Maccy 内容表失败：{error}"))?;
    let mut contents = Vec::new();
    let mut skipped_large = false;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let stored_len: i64 = row.get(2).unwrap_or(0);
        let Some(bytes) = row.get::<_, Option<Vec<u8>>>(1).unwrap_or(None) else {
            skipped_large |= stored_len > MAX_CONTENT_BYTES;
            continue;
        };
        contents.push(MaccyContent {
            data_type: row.get(0).unwrap_or_default(),
            bytes,
        });
    }
    let warning = skipped_large.then(|| "已跳过超过 64 MB 的单个 Maccy 剪贴板格式".to_string());
    Ok((contents, warning))
}

fn candidates_from_contents(item: &MaccyItem, contents: Vec<MaccyContent>) -> ImportedClip {
    let mut by_type = HashMap::<String, Vec<&[u8]>>::new();
    for content in &contents {
        by_type
            .entry(content.data_type.trim().to_ascii_lowercase())
            .or_default()
            .push(&content.bytes);
    }

    let file_paths = values_for_types(&by_type, TYPE_FILE_URL)
        .into_iter()
        .filter_map(decode_file_url)
        .collect::<Vec<_>>();
    if !file_paths.is_empty() {
        let candidates = file_paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| {
                candidate(
                    item,
                    file_content_type(&path),
                    path,
                    None,
                    item.created_at,
                    item.sort_at.saturating_add(index as i64),
                    true,
                )
            })
            .collect();
        return ImportedClip {
            candidates,
            warning: None,
        };
    }

    for image_types in [TYPE_PNG, TYPE_JPEG, TYPE_TIFF, TYPE_HEIC] {
        if let Some(bytes) = first_value_for_types(&by_type, image_types) {
            if let Some(data_url) = image_bytes_to_png_data_url(bytes) {
                return ImportedClip {
                    candidates: vec![candidate(
                        item,
                        "image".to_string(),
                        data_url,
                        None,
                        item.created_at,
                        item.sort_at,
                        false,
                    )],
                    warning: None,
                };
            }
        }
    }

    let plain_text = first_value_for_types(&by_type, TYPE_STRING)
        .and_then(decode_text)
        .filter(|text| !text.is_empty());
    let html = first_value_for_types(&by_type, TYPE_HTML)
        .and_then(decode_text)
        .filter(|html| !html.trim().is_empty());
    let rtf = first_value_for_types(&by_type, TYPE_RTF);

    if html.is_some() || rtf.is_some() {
        let mut text = plain_text.clone().unwrap_or_default();
        if text.is_empty() {
            text = html
                .as_deref()
                .map(|html| derive_rich_text_content("", Some(html)))
                .unwrap_or_default();
        }
        if text.is_empty() {
            text = rtf
                .map(super::ditto::plain_text_from_rtf)
                .unwrap_or_default();
        }
        if text.is_empty() {
            text = item.title.trim().to_string();
        }
        if !text.is_empty() {
            let base_html = html.unwrap_or_else(|| html_from_plain_text(&text));
            let named_formats = rtf
                .filter(|bytes| !bytes.is_empty())
                .map(|bytes| {
                    vec![NamedClipboardFormat {
                        name: "Rich Text Format".to_string(),
                        data: bytes.to_vec(),
                    }]
                })
                .unwrap_or_default();
            let stored_html = attach_rich_named_formats(&base_html, &named_formats);
            return ImportedClip {
                candidates: vec![candidate(
                    item,
                    "rich_text".to_string(),
                    text,
                    Some(stored_html),
                    item.created_at,
                    item.sort_at,
                    false,
                )],
                warning: None,
            };
        }
    }

    if let Some(text) = plain_text {
        return ImportedClip {
            candidates: vec![candidate(
                item,
                detect_content_type(&text),
                text,
                None,
                item.created_at,
                item.sort_at,
                false,
            )],
            warning: None,
        };
    }

    let fallback = item.title.trim();
    if !fallback.is_empty() {
        return ImportedClip {
            candidates: vec![candidate(
                item,
                "text".to_string(),
                fallback.to_string(),
                None,
                item.created_at,
                item.sort_at,
                false,
            )],
            warning: Some("该 Maccy 条目的原始格式无法恢复，已保留其可搜索标题".to_string()),
        };
    }

    let types = contents
        .iter()
        .map(|content| content.data_type.trim())
        .filter(|value| !value.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join(", ");
    ImportedClip {
        candidates: Vec::new(),
        warning: Some(if types.is_empty() {
            "Maccy 条目不包含可导入的数据".to_string()
        } else {
            format!("不支持的 Maccy 条目格式：{types}")
        }),
    }
}

fn candidate(
    item: &MaccyItem,
    content_type: String,
    content: String,
    html_content: Option<String>,
    created_at: i64,
    sort_at: i64,
    is_external: bool,
) -> ImportCandidate {
    let pin = item.pin.as_deref().unwrap_or_default().trim();
    let is_pinned = item.pin.is_some();
    ImportCandidate {
        content_type,
        content,
        html_content,
        created_at,
        sort_at,
        source_app: if item.application.trim().is_empty() {
            "Maccy".to_string()
        } else {
            item.application.trim().to_string()
        },
        is_pinned,
        pinned_order: if is_pinned { sort_at } else { 0 },
        tags: Vec::new(),
        note: String::new(),
        use_count: item.use_count,
        item_hotkey: pin.to_ascii_uppercase(),
        item_hotkey_global: false,
        last_used_at: 0,
        move_to_group_hotkey: String::new(),
        move_to_group_hotkey_global: false,
        is_external,
    }
}

fn first_value_for_types<'a>(
    values: &'a HashMap<String, Vec<&'a [u8]>>,
    types: &[&str],
) -> Option<&'a [u8]> {
    types.iter().find_map(|data_type| {
        values
            .get(&data_type.to_ascii_lowercase())
            .and_then(|values| values.first().copied())
    })
}

fn values_for_types<'a>(
    values: &'a HashMap<String, Vec<&'a [u8]>>,
    types: &[&str],
) -> Vec<&'a [u8]> {
    types
        .iter()
        .flat_map(|data_type| {
            values
                .get(&data_type.to_ascii_lowercase())
                .into_iter()
                .flatten()
                .copied()
        })
        .collect()
}

fn decode_text(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xff, 0xfe]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).ok().map(trim_trailing_nuls);
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).ok().map(trim_trailing_nuls);
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    Some(trim_trailing_nuls(
        String::from_utf8_lossy(bytes).into_owned(),
    ))
}

fn trim_trailing_nuls(mut value: String) -> String {
    while value.ends_with('\0') {
        value.pop();
    }
    value
}

fn decode_file_url(bytes: &[u8]) -> Option<String> {
    let value = decode_text(bytes)?;
    let value = value.trim();
    let encoded_path = value
        .strip_prefix("file://localhost")
        .or_else(|| value.strip_prefix("file://"))
        .unwrap_or(value);
    let decoded = urlencoding::decode(encoded_path).ok()?.into_owned();
    (!decoded.trim().is_empty()).then_some(decoded)
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

#[cfg(target_os = "macos")]
fn image_bytes_to_png_data_url(bytes: &[u8]) -> Option<String> {
    autoreleasepool(|_| {
        let source = NSData::from_vec(bytes.to_vec());
        let bitmap = NSBitmapImageRep::imageRepWithData(&source)?;
        let properties = NSDictionary::<NSBitmapImageRepPropertyKey, AnyObject>::from_slices(
            &[] as &[&NSBitmapImageRepPropertyKey],
            &[] as &[&AnyObject],
        );
        let png = unsafe {
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
        }?;
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.to_vec())
        ))
    })
}

#[cfg(not(target_os = "macos"))]
fn image_bytes_to_png_data_url(bytes: &[u8]) -> Option<String> {
    let image = image::load_from_memory(bytes).ok()?;
    let mut output = std::io::Cursor::new(Vec::new());
    image.write_to(&mut output, image::ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(output.into_inner())
    ))
}

fn combine_warnings(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) if first != second => Some(format!("{first}；{second}")),
        (Some(first), _) => Some(first),
        (_, Some(second)) => Some(second),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_core_data_reference_dates() {
        assert_eq!(
            core_data_date_to_unix_ms(721_692_800.125),
            1_700_000_000_125
        );
    }

    #[test]
    fn decodes_maccy_file_urls() {
        assert_eq!(
            decode_file_url(b"file:///Users/Test/My%20File.txt"),
            Some("/Users/Test/My File.txt".to_string())
        );
    }

    #[test]
    fn converts_maccy_images_to_png_data_urls() {
        let png = base64::engine::general_purpose::STANDARD
            .decode(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
            )
            .expect("decode fixture");
        let converted = image_bytes_to_png_data_url(&png).expect("convert image");
        assert!(converted.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn empty_maccy_pin_still_marks_the_item_as_pinned() {
        let item = MaccyItem {
            id: 1,
            created_at: 1,
            sort_at: 2,
            use_count: 1,
            pin: Some(String::new()),
            title: String::new(),
            application: String::new(),
        };
        let imported = candidate(
            &item,
            "text".to_string(),
            "Pinned".to_string(),
            None,
            1,
            2,
            false,
        );
        assert!(imported.is_pinned);
        assert_eq!(imported.item_hotkey, "");
    }
}
