use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameW, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GHND};

// Clipboard format constants
const CF_DIB: u32 = 8; // DIB
const CF_DIBV5: u32 = 17; // DIBV5
const CF_UNICODETEXT: u32 = 13; // Unicode text

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct BITMAPINFOHEADER {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

fn read_u32_le(raw_data: &[u8], offset: usize) -> Option<u32> {
    let bytes = raw_data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u16_le(raw_data: &[u8], offset: usize) -> Option<u16> {
    let bytes = raw_data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_i32_le(raw_data: &[u8], offset: usize) -> Option<i32> {
    read_u32_le(raw_data, offset).map(|value| i32::from_le_bytes(value.to_le_bytes()))
}

fn masked_component(pixel: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max_value = mask >> shift;
    let value = (pixel & mask) >> shift;
    if max_value == 0 {
        0
    } else {
        (((u64::from(value) * 255) + u64::from(max_value) / 2) / u64::from(max_value)) as u8
    }
}

fn redundant_extended_masks_size(
    raw_data: &[u8],
    offset: usize,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    alpha_mask: u32,
) -> usize {
    if read_u32_le(raw_data, offset) != Some(red_mask)
        || read_u32_le(raw_data, offset + 4) != Some(green_mask)
        || read_u32_le(raw_data, offset + 8) != Some(blue_mask)
    {
        return 0;
    }

    if alpha_mask != 0 && read_u32_le(raw_data, offset + 12) == Some(alpha_mask) {
        16
    } else {
        12
    }
}

#[derive(Clone)]
pub struct ImageData {
    pub width: usize,
    pub height: usize,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedClipboardFormat {
    pub name: String,
    pub data: Vec<u8>,
}

unsafe fn copy_hglobal_bytes(h_data: windows::Win32::Foundation::HANDLE) -> Option<Vec<u8>> {
    const MAX_CLIPBOARD_GLOBAL_BYTES: usize = 256 * 1024 * 1024;

    if h_data.is_invalid() {
        return None;
    }

    let h_global = HGLOBAL(h_data.0 as *mut _);
    let p_data = GlobalLock(h_global);
    if p_data.is_null() {
        return None;
    }

    let data_size = GlobalSize(h_global);
    if data_size == 0 || data_size > MAX_CLIPBOARD_GLOBAL_BYTES {
        let _ = GlobalUnlock(h_global);
        return None;
    }
    let mut buffer = vec![0u8; data_size];
    std::ptr::copy_nonoverlapping(p_data as *const u8, buffer.as_mut_ptr(), data_size);
    let _ = GlobalUnlock(h_global);
    Some(buffer)
}

unsafe fn clipboard_format_name(format_id: u32) -> Option<String> {
    let mut buffer = vec![0u16; 256];
    let len = GetClipboardFormatNameW(format_id, &mut buffer);
    if len <= 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..len as usize]))
}

unsafe fn set_named_clipboard_format_bytes(format_id: u32, data: &[u8]) -> Result<(), String> {
    let alloc_len = data.len().max(1);
    let h_global = GlobalAlloc(GHND, alloc_len).map_err(|e| e.to_string())?;
    let p_mem = GlobalLock(h_global);
    if p_mem.is_null() {
        return Err("GlobalLock failed".to_string());
    }

    if !data.is_empty() {
        std::ptr::copy_nonoverlapping(data.as_ptr(), p_mem as *mut u8, data.len());
    } else {
        *(p_mem as *mut u8) = 0;
    }

    let _ = GlobalUnlock(h_global);
    SetClipboardData(
        format_id,
        Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

const MAX_CLIPBOARD_IMAGE_PIXELS: usize = 32 * 1024 * 1024;
const MAX_CLIPBOARD_IMAGE_DIMENSION: usize = 32_768;

fn decode_dib_bytes(raw_data: &[u8]) -> Option<ImageData> {
    const BI_RGB: u32 = 0;
    const BI_BITFIELDS: u32 = 3;
    const BI_ALPHABITFIELDS: u32 = 6;

    if raw_data.len() < std::mem::size_of::<BITMAPINFOHEADER>() {
        return None;
    }

    let header_size = read_u32_le(raw_data, 0)? as usize;
    if header_size < std::mem::size_of::<BITMAPINFOHEADER>() || header_size > raw_data.len() {
        return None;
    }

    let width_raw = read_i32_le(raw_data, 4)?;
    let height_raw = read_i32_le(raw_data, 8)?;
    if width_raw <= 0 || height_raw == 0 {
        return None;
    }
    let width = usize::try_from(width_raw).ok()?;
    let height = usize::try_from(height_raw.checked_abs()?).ok()?;
    if width > MAX_CLIPBOARD_IMAGE_DIMENSION || height > MAX_CLIPBOARD_IMAGE_DIMENSION {
        return None;
    }
    let pixel_count = width.checked_mul(height)?;
    if pixel_count == 0 || pixel_count > MAX_CLIPBOARD_IMAGE_PIXELS {
        return None;
    }

    if read_u16_le(raw_data, 12)? != 1 {
        return None;
    }
    let bit_count = usize::from(read_u16_le(raw_data, 14)?);
    if !matches!(bit_count, 1 | 4 | 8 | 16 | 24 | 32) {
        return None;
    }
    let compression = read_u32_le(raw_data, 16)?;
    if !matches!(compression, BI_RGB | BI_BITFIELDS | BI_ALPHABITFIELDS)
        || (bit_count <= 8 && compression != BI_RGB)
    {
        return None;
    }

    let mut external_mask_size = 0usize;
    let (red_mask, green_mask, blue_mask, alpha_mask) =
        if matches!(compression, BI_BITFIELDS | BI_ALPHABITFIELDS) {
            if !matches!(bit_count, 16 | 24 | 32) {
                return None;
            }

            if header_size >= 52 {
                (
                    read_u32_le(raw_data, 40)?,
                    read_u32_le(raw_data, 44)?,
                    read_u32_le(raw_data, 48)?,
                    if header_size >= 56 {
                        read_u32_le(raw_data, 52).unwrap_or(0)
                    } else {
                        0
                    },
                )
            } else if header_size == 40 {
                let red = read_u32_le(raw_data, header_size)?;
                let green = read_u32_le(raw_data, header_size + 4)?;
                let blue = read_u32_le(raw_data, header_size + 8)?;
                let has_alpha = compression == BI_ALPHABITFIELDS;
                let possible_alpha = if has_alpha {
                    read_u32_le(raw_data, header_size + 12)?
                } else {
                    0
                };
                external_mask_size = if has_alpha { 16 } else { 12 };
                (red, green, blue, if has_alpha { possible_alpha } else { 0 })
            } else {
                return None;
            }
        } else {
            match bit_count {
                16 => (0x00007c00, 0x000003e0, 0x0000001f, 0),
                24 | 32 => (0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000),
                _ => (0, 0, 0, 0),
            }
        };

    if matches!(compression, BI_BITFIELDS | BI_ALPHABITFIELDS)
        && (red_mask == 0 || green_mask == 0 || blue_mask == 0)
    {
        return None;
    }

    let redundant_mask_size =
        if header_size > 40 && matches!(compression, BI_BITFIELDS | BI_ALPHABITFIELDS) {
            redundant_extended_masks_size(
                raw_data,
                header_size,
                red_mask,
                green_mask,
                blue_mask,
                alpha_mask,
            )
        } else {
            0
        };

    let palette_entries = if bit_count <= 8 {
        let declared = read_u32_le(raw_data, 32)? as usize;
        let maximum = 1usize.checked_shl(bit_count as u32)?;
        if declared == 0 {
            maximum
        } else if declared <= maximum {
            declared
        } else {
            return None;
        }
    } else {
        0
    };
    let palette_size = palette_entries.checked_mul(4)?;
    let palette_offset = header_size
        .checked_add(external_mask_size)?
        .checked_add(redundant_mask_size)?;
    let pixel_data_offset = palette_offset.checked_add(palette_size)?;
    if pixel_data_offset > raw_data.len() {
        return None;
    }

    let row_bits = width.checked_mul(bit_count)?;
    let row_stride_formula = row_bits.checked_add(31)?.checked_div(32)?.checked_mul(4)?;
    let available = raw_data.len().checked_sub(pixel_data_offset)?;
    let size_image = read_u32_le(raw_data, 20)? as usize;
    let row_stride_from_header = if size_image > 0 && size_image % height == 0 {
        let candidate = size_image / height;
        let candidate_size = candidate.checked_mul(height)?;
        if candidate >= row_stride_formula
            && candidate <= row_stride_formula.saturating_add(4096)
            && candidate_size <= available
        {
            Some(candidate)
        } else {
            None
        }
    } else {
        None
    };
    let row_stride = row_stride_from_header.unwrap_or(row_stride_formula);
    let required_pixel_bytes = row_stride.checked_mul(height)?;
    if required_pixel_bytes > available {
        return None;
    }

    let rgba_len = pixel_count.checked_mul(4)?;
    let mut rgba_data = vec![0u8; rgba_len];
    let raw_bi_rgb_alpha = bit_count == 32 && compression == BI_RGB;
    let mut alpha_non_zero = false;

    for y in 0..height {
        let source_y = if height_raw > 0 { height - 1 - y } else { y };
        let row_start = pixel_data_offset.checked_add(source_y.checked_mul(row_stride)?)?;
        let row_end = row_start.checked_add(row_stride)?;
        let row = raw_data.get(row_start..row_end)?;

        for x in 0..width {
            let destination = y.checked_mul(width)?.checked_add(x)?.checked_mul(4)?;
            let (red, green, blue, alpha) = match bit_count {
                1 | 4 | 8 => {
                    let palette_index = match bit_count {
                        1 => (row.get(x / 8)? >> (7 - (x % 8))) & 0x01,
                        4 => {
                            let byte = *row.get(x / 2)?;
                            if x % 2 == 0 {
                                byte >> 4
                            } else {
                                byte & 0x0f
                            }
                        }
                        8 => *row.get(x)?,
                        _ => unreachable!(),
                    } as usize;
                    if palette_index >= palette_entries {
                        return None;
                    }
                    let color_offset = palette_offset.checked_add(palette_index.checked_mul(4)?)?;
                    let color = raw_data.get(color_offset..color_offset + 4)?;
                    (color[2], color[1], color[0], 255)
                }
                16 => {
                    let offset = x.checked_mul(2)?;
                    let bytes = row.get(offset..offset + 2)?;
                    let pixel = u16::from_le_bytes([bytes[0], bytes[1]]) as u32;
                    (
                        masked_component(pixel, red_mask),
                        masked_component(pixel, green_mask),
                        masked_component(pixel, blue_mask),
                        if alpha_mask == 0 {
                            255
                        } else {
                            masked_component(pixel, alpha_mask)
                        },
                    )
                }
                24 => {
                    let offset = x.checked_mul(3)?;
                    let bytes = row.get(offset..offset + 3)?;
                    if compression == BI_RGB {
                        (bytes[2], bytes[1], bytes[0], 255)
                    } else {
                        let pixel = u32::from(bytes[0])
                            | (u32::from(bytes[1]) << 8)
                            | (u32::from(bytes[2]) << 16);
                        (
                            masked_component(pixel, red_mask),
                            masked_component(pixel, green_mask),
                            masked_component(pixel, blue_mask),
                            if alpha_mask == 0 {
                                255
                            } else {
                                masked_component(pixel, alpha_mask)
                            },
                        )
                    }
                }
                32 => {
                    let offset = x.checked_mul(4)?;
                    let bytes = row.get(offset..offset + 4)?;
                    let pixel = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    let alpha = if raw_bi_rgb_alpha {
                        bytes[3]
                    } else if alpha_mask == 0 {
                        255
                    } else {
                        masked_component(pixel, alpha_mask)
                    };
                    (
                        masked_component(pixel, red_mask),
                        masked_component(pixel, green_mask),
                        masked_component(pixel, blue_mask),
                        alpha,
                    )
                }
                _ => return None,
            };

            rgba_data[destination] = red;
            rgba_data[destination + 1] = green;
            rgba_data[destination + 2] = blue;
            rgba_data[destination + 3] = alpha;
            alpha_non_zero |= alpha != 0;
        }
    }

    if raw_bi_rgb_alpha && !alpha_non_zero {
        for alpha in rgba_data.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
    }

    Some(ImageData {
        width,
        height,
        bytes: rgba_data,
    })
}

/// Try to get image from Windows clipboard using native API.
pub unsafe fn get_clipboard_image() -> Option<ImageData> {
    if OpenClipboard(None).is_err() {
        return None;
    }

    let raw_data = (|| {
        let handle = match GetClipboardData(CF_DIBV5) {
            Ok(handle) if !handle.is_invalid() => handle,
            _ => match GetClipboardData(CF_DIB) {
                Ok(handle) if !handle.is_invalid() => handle,
                _ => return None,
            },
        };
        copy_hglobal_bytes(handle)
    })();
    let _ = CloseClipboard();

    raw_data.as_deref().and_then(decode_dib_bytes)
}

#[cfg(test)]
mod tests {
    use super::decode_dib_bytes;

    fn dib_header(
        width: i32,
        height: i32,
        bit_count: u16,
        compression: u32,
        image_size: u32,
        color_count: u32,
    ) -> Vec<u8> {
        let mut header = vec![0u8; 40];
        header[0..4].copy_from_slice(&40u32.to_le_bytes());
        header[4..8].copy_from_slice(&width.to_le_bytes());
        header[8..12].copy_from_slice(&height.to_le_bytes());
        header[12..14].copy_from_slice(&1u16.to_le_bytes());
        header[14..16].copy_from_slice(&bit_count.to_le_bytes());
        header[16..20].copy_from_slice(&compression.to_le_bytes());
        header[20..24].copy_from_slice(&image_size.to_le_bytes());
        header[32..36].copy_from_slice(&color_count.to_le_bytes());
        header
    }

    #[test]
    fn decodes_32bit_bi_rgb_and_repairs_zero_alpha() {
        let mut dib = dib_header(2, 1, 32, 0, 8, 0);
        dib.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0]);

        let image = decode_dib_bytes(&dib).expect("32-bit DIB should decode");
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.bytes, vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn decodes_top_down_24bit_rows_with_padding() {
        let mut dib = dib_header(1, -2, 24, 0, 8, 0);
        dib.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);

        let image = decode_dib_bytes(&dib).expect("24-bit DIB should decode");
        assert_eq!((image.width, image.height), (1, 2));
        assert_eq!(image.bytes, vec![255, 0, 0, 255, 0, 0, 255, 255]);
    }

    #[test]
    fn decodes_16bit_bitfields() {
        let mut dib = dib_header(1, 1, 16, 3, 4, 0);
        dib.extend_from_slice(&0x0000f800u32.to_le_bytes());
        dib.extend_from_slice(&0x000007e0u32.to_le_bytes());
        dib.extend_from_slice(&0x0000001fu32.to_le_bytes());
        dib.extend_from_slice(&[0x00, 0xf8, 0x00, 0x00]);

        let image = decode_dib_bytes(&dib).expect("16-bit bitfields DIB should decode");
        assert_eq!(image.bytes, vec![255, 0, 0, 255]);
    }

    #[test]
    fn rejects_invalid_or_oversized_dimensions() {
        assert!(decode_dib_bytes(&dib_header(0, 1, 32, 0, 0, 0)).is_none());
        assert!(decode_dib_bytes(&dib_header(i32::MAX, 1, 32, 0, 0, 0)).is_none());
        assert!(decode_dib_bytes(&dib_header(1, i32::MIN, 32, 0, 0, 0)).is_none());
    }
}
const CF_HDROP: u32 = 15;

/// Try to get file paths from Windows clipboard (CF_HDROP)
pub unsafe fn get_clipboard_files() -> Option<Vec<String>> {
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    if OpenClipboard(None).is_err() {
        return None;
    }

    let result = (|| {
        let h_drop = match GetClipboardData(CF_HDROP) {
            Ok(handle) if !handle.is_invalid() => handle,
            _ => return None,
        };

        let h_drop = HDROP(h_drop.0 as _);
        let file_count = DragQueryFileW(h_drop, u32::MAX, None);
        let mut files = Vec::new();
        for index in 0..file_count {
            let path_len = DragQueryFileW(h_drop, index, None) as usize;
            if path_len == 0 {
                continue;
            }
            let mut path = vec![0u16; path_len + 1];
            let copied_len = DragQueryFileW(h_drop, index, Some(&mut path)) as usize;
            if copied_len > 0 {
                files.push(String::from_utf16_lossy(&path[..copied_len]));
            }
        }

        if files.is_empty() {
            None
        } else {
            Some(files)
        }
    })();

    let _ = CloseClipboard();
    result
}

/// Set files to Windows clipboard (CF_HDROP)
pub unsafe fn set_clipboard_files(paths: Vec<String>) -> Result<(), String> {
    if OpenClipboard(None).is_err() {
        return Err("Cannot open clipboard".into());
    }

    // Prepare payload (Double null terminated list of wide strings)
    let mut buffer: Vec<u16> = Vec::new();
    for path in paths {
        buffer.extend(path.encode_utf16());
        buffer.push(0);
    }
    buffer.push(0); // Double null terminator

    // Calculate size needed
    // DROPFILES struct size + buffer size in bytes
    // pFiles(4) + pt.x(4) + pt.y(4) + fNC(4) + fWide(4) = 20 bytes
    let dropfiles_size = 20;
    let buffer_size = buffer.len() * 2;
    let total_size = dropfiles_size + buffer_size;

    let h_global = GlobalAlloc(GHND, total_size).map_err(|e| e.to_string())?;

    let p_mem = GlobalLock(h_global);
    if p_mem.is_null() {
        let _ = CloseClipboard();
        return Err("GlobalLock failed".into());
    }

    // Write DROPFILES struct
    // Offset 0: pFiles = 20 (size of struct)
    *(p_mem as *mut u32) = 20;

    // Offset 16: fWide = 1
    *(p_mem.add(16) as *mut i32) = 1;

    // Write file paths
    let p_files = p_mem.add(20) as *mut u16;
    std::ptr::copy_nonoverlapping(buffer.as_ptr(), p_files, buffer.len());

    let _ = GlobalUnlock(h_global);

    let _ = EmptyClipboard();
    if SetClipboardData(
        CF_HDROP,
        Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
    )
    .is_err()
    {
        let _ = CloseClipboard();
        return Err("SetClipboardData failed".into());
    }

    let _ = CloseClipboard();
    Ok(())
}

pub fn get_clipboard_sequence_number() -> u32 {
    unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

pub unsafe fn clear_clipboard() -> Result<(), String> {
    if OpenClipboard(None).is_err() {
        return Err("Cannot open clipboard".into());
    }

    let result = EmptyClipboard().map_err(|e| e.to_string());
    let _ = CloseClipboard();
    result.map(|_| ())
}

/// Get raw bytes from a specific clipboard format by name
pub unsafe fn get_clipboard_raw_format(format_name: &str) -> Option<Vec<u8>> {
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;

    if OpenClipboard(None).is_err() {
        return None;
    }

    let result = (|| {
        let name_w: Vec<u16> = format_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let format_id = RegisterClipboardFormatW(windows::core::PCWSTR(name_w.as_ptr()));
        if format_id == 0 {
            return None;
        }

        let h_data = match GetClipboardData(format_id) {
            Ok(handle) => handle,
            Err(_) => return None,
        };

        copy_hglobal_bytes(h_data)
    })();

    let _ = CloseClipboard();
    result
}

/// Enumerate registered clipboard formats with HGLOBAL-backed payloads.
pub unsafe fn get_named_clipboard_formats(
    max_formats: usize,
    max_format_bytes: usize,
    max_total_bytes: usize,
) -> Vec<NamedClipboardFormat> {
    if OpenClipboard(None).is_err() {
        return Vec::new();
    }

    let result = (|| {
        let mut formats = Vec::new();
        let mut total_bytes = 0usize;
        let mut current_id = 0u32;

        loop {
            current_id = EnumClipboardFormats(current_id);
            if current_id == 0 {
                break;
            }

            let Some(name) = clipboard_format_name(current_id) else {
                continue;
            };

            let h_data = match GetClipboardData(current_id) {
                Ok(handle) => handle,
                Err(_) => continue,
            };

            let Some(data) = copy_hglobal_bytes(h_data) else {
                continue;
            };

            if data.is_empty() || data.len() > max_format_bytes {
                continue;
            }

            if total_bytes.saturating_add(data.len()) > max_total_bytes {
                continue;
            }

            total_bytes = total_bytes.saturating_add(data.len());
            formats.push(NamedClipboardFormat { name, data });
            if formats.len() >= max_formats {
                break;
            }
        }

        formats
    })();

    let _ = CloseClipboard();
    result
}

unsafe fn set_clipboard_text_and_html_inner(
    text: &str,
    cf_html: &str,
    clear_existing: bool,
) -> Result<(), String> {
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;

    if OpenClipboard(None).is_err() {
        return Err("Cannot open clipboard".into());
    }

    let result = (|| {
        if clear_existing {
            let _ = EmptyClipboard();
        }

        // 1) Set CF_UNICODETEXT
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        let byte_len = wide.len() * 2;
        let h_text = GlobalAlloc(GHND, byte_len).map_err(|e| e.to_string())?;
        let p_text = GlobalLock(h_text);
        if p_text.is_null() {
            return Err("GlobalLock failed".to_string());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p_text as *mut u8, byte_len);
        let _ = GlobalUnlock(h_text);
        if SetClipboardData(
            CF_UNICODETEXT,
            Some(windows::Win32::Foundation::HANDLE(h_text.0 as _)),
        )
        .is_err()
        {
            return Err("SetClipboardData (CF_UNICODETEXT) failed".to_string());
        }

        // 2) Set CF_HTML
        let format_name = "HTML Format";
        let name_w: Vec<u16> = format_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let format_id = RegisterClipboardFormatW(windows::core::PCWSTR(name_w.as_ptr()));
        if format_id == 0 {
            return Err("RegisterClipboardFormatW failed".to_string());
        }

        let html_bytes = cf_html.as_bytes();
        let h_html = GlobalAlloc(GHND, html_bytes.len() + 1).map_err(|e| e.to_string())?;
        let p_html = GlobalLock(h_html);
        if p_html.is_null() {
            return Err("GlobalLock failed".to_string());
        }
        std::ptr::copy_nonoverlapping(html_bytes.as_ptr(), p_html as *mut u8, html_bytes.len());
        *(p_html.add(html_bytes.len()) as *mut u8) = 0;
        let _ = GlobalUnlock(h_html);
        let _ = SetClipboardData(
            format_id,
            Some(windows::Win32::Foundation::HANDLE(h_html.0 as _)),
        );

        Ok(())
    })();

    let _ = CloseClipboard();
    result
}

/// Set Unicode text and CF_HTML, replacing existing clipboard formats.
pub unsafe fn set_clipboard_text_and_html(text: &str, cf_html: &str) -> Result<(), String> {
    set_clipboard_text_and_html_inner(text, cf_html, true)
}

/// Append/override Unicode text and CF_HTML while keeping existing non-text formats (e.g. image/DIB).
pub unsafe fn append_clipboard_text_and_html(text: &str, cf_html: &str) -> Result<(), String> {
    set_clipboard_text_and_html_inner(text, cf_html, false)
}

/// Append registered clipboard formats while keeping existing data intact.
pub unsafe fn append_named_clipboard_formats(
    formats: &[NamedClipboardFormat],
) -> Result<(), String> {
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;

    if formats.is_empty() {
        return Ok(());
    }

    if OpenClipboard(None).is_err() {
        return Err("Cannot open clipboard".into());
    }

    let result = (|| {
        for format in formats {
            if format.name.trim().is_empty() {
                continue;
            }

            let name_w: Vec<u16> = format
                .name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let format_id = RegisterClipboardFormatW(windows::core::PCWSTR(name_w.as_ptr()));
            if format_id == 0 {
                continue;
            }

            set_named_clipboard_format_bytes(format_id, &format.data)?;
        }

        Ok(())
    })();

    let _ = CloseClipboard();
    result
}

/// Set image (DIB) and optionally a raw format (like GIF) to clipboard in one go
pub unsafe fn set_clipboard_image_and_gif(
    image: ImageData,
    raw_format_name: Option<&str>,
    raw_data: Option<&[u8]>,
) -> Result<(), String> {
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;

    if OpenClipboard(None).is_err() {
        return Err("Cannot open clipboard".into());
    }

    let result = (|| {
        let _ = EmptyClipboard();

        // 1. Set Raw Format (e.g. "GIF")
        if let (Some(primary_name), Some(data)) = (raw_format_name, raw_data) {
            let names = if primary_name == "GIF" {
                vec![
                    "GIF",
                    "Animated GIF",
                    "gif",
                    "image/gif",
                    "Graphics Interchange Format",
                ]
            } else {
                vec![primary_name]
            };

            for name in names {
                let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let format_id = RegisterClipboardFormatW(windows::core::PCWSTR(name_w.as_ptr()));
                if format_id != 0 {
                    if let Ok(h_global) = GlobalAlloc(GHND, data.len()) {
                        let p_mem = GlobalLock(h_global);
                        if !p_mem.is_null() {
                            std::ptr::copy_nonoverlapping(
                                data.as_ptr(),
                                p_mem as *mut u8,
                                data.len(),
                            );
                            let _ = GlobalUnlock(h_global);
                            let _ = SetClipboardData(
                                format_id,
                                Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
                            );
                        }
                    }
                }
            }
        }

        // 2. Set CF_DIB
        // DIB data is BITMAPINFOHEADER + Pixel Data (BGRA, bottom-up)
        let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
        let pixel_data_size = image.width * image.height * 4;
        let total_size = header_size + pixel_data_size;

        let h_global = GlobalAlloc(GHND, total_size).map_err(|e| e.to_string())?;
        let p_mem = GlobalLock(h_global);
        if p_mem.is_null() {
            return Err("GlobalLock failed".to_string());
        }

        let header = BITMAPINFOHEADER {
            bi_size: header_size as u32,
            bi_width: image.width as i32,
            bi_height: image.height as i32, // Top-down if positive in some contexts, but CF_DIB is usually bottom-up
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: 0, // BI_RGB
            bi_size_image: pixel_data_size as u32,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        };

        // Write header
        std::ptr::copy_nonoverlapping(
            &header as *const _ as *const u8,
            p_mem as *mut u8,
            header_size,
        );

        // Write pixel data (Convert RGBA to BGRA and flip vertically for DIB)
        let p_pixels = p_mem.add(header_size) as *mut u8;
        for y in 0..image.height {
            let src_y = image.height - 1 - y; // Flip vertically
            let src_offset = src_y * image.width * 4;
            let dst_offset = y * image.width * 4;

            for x in 0..image.width {
                let s = src_offset + x * 4;
                let d = dst_offset + x * 4;
                // RGBA -> BGRA
                *p_pixels.add(d) = image.bytes[s + 2]; // B
                *p_pixels.add(d + 1) = image.bytes[s + 1]; // G
                *p_pixels.add(d + 2) = image.bytes[s]; // R
                *p_pixels.add(d + 3) = image.bytes[s + 3]; // A
            }
        }

        let _ = GlobalUnlock(h_global);
        if SetClipboardData(
            CF_DIB,
            Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
        )
        .is_err()
        {
            return Err("SetClipboardData (CF_DIB) failed".to_string());
        }

        Ok(())
    })();

    let _ = CloseClipboard();
    result
}

unsafe fn set_clipboard_unicode_text_locked(text: &str) -> Result<(), String> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let byte_len = wide.len() * 2;
    let h_text = GlobalAlloc(GHND, byte_len).map_err(|e| e.to_string())?;
    let p_text = GlobalLock(h_text);
    if p_text.is_null() {
        return Err("GlobalLock failed".to_string());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p_text as *mut u8, byte_len);
    let _ = GlobalUnlock(h_text);
    if SetClipboardData(
        CF_UNICODETEXT,
        Some(windows::Win32::Foundation::HANDLE(h_text.0 as _)),
    )
    .is_err()
    {
        return Err("SetClipboardData (CF_UNICODETEXT) failed".to_string());
    }
    Ok(())
}

unsafe fn set_clipboard_hdrop_path_locked(path: &str) -> Result<(), String> {
    let mut buffer: Vec<u16> = path.encode_utf16().collect();
    buffer.push(0);
    buffer.push(0);

    let dropfiles_size = 20;
    let buffer_size = buffer.len() * 2;
    let total_size = dropfiles_size + buffer_size;

    let h_global = GlobalAlloc(GHND, total_size).map_err(|e| e.to_string())?;
    let p_mem = GlobalLock(h_global);
    if p_mem.is_null() {
        return Err("GlobalLock failed".to_string());
    }

    *(p_mem as *mut u32) = 20;
    *(p_mem.add(16) as *mut i32) = 1;
    let p_files = p_mem.add(20) as *mut u16;
    std::ptr::copy_nonoverlapping(buffer.as_ptr(), p_files, buffer.len());

    let _ = GlobalUnlock(h_global);
    if SetClipboardData(
        CF_HDROP,
        Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
    )
    .is_err()
    {
        return Err("SetClipboardData (CF_HDROP) failed".to_string());
    }
    Ok(())
}

/// Set image with multiple formats: GIF (optional), PNG (optional), DIB, and optional file path.
/// When `file_path` is set, also publishes CF_HDROP + CF_UNICODETEXT for path-aware apps (#134).
pub unsafe fn set_clipboard_image_with_formats(
    image: ImageData,
    gif_data: Option<&[u8]>,
    png_data: Option<&[u8]>,
    file_path: Option<&str>,
) -> Result<Option<String>, String> {
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;

    // For GIF without an existing path, create a temp file before opening the clipboard.
    let gif_temp_path: Option<String> = if gif_data.is_some() && file_path.is_none() {
        let temp_dir = std::env::temp_dir();
        let filename = format!(
            "TieZ_GIF_{}.gif",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let path = temp_dir.join(filename);
        if let Some(gif_bytes) = gif_data {
            if std::fs::write(&path, gif_bytes).is_ok() {
                path.to_str().map(|s| s.to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let hdrop_path = file_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .or(gif_temp_path.clone());

    if OpenClipboard(None).is_err() {
        return Err("Cannot open clipboard".into());
    }

    let result = (|| {
        let _ = EmptyClipboard();

        // 1. File reference + path text for apps that cannot consume raw bitmap data (#134).
        if let Some(ref path) = hdrop_path {
            set_clipboard_hdrop_path_locked(path)?;
            set_clipboard_unicode_text_locked(path)?;
        }

        // 2. Set GIF formats (if available)
        if let Some(gif_bytes) = gif_data {
            let gif_format_names = [
                "GIF",
                "Animated GIF",
                "gif",
                "image/gif",
                "Graphics Interchange Format",
            ];

            for name in gif_format_names {
                let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let format_id = RegisterClipboardFormatW(windows::core::PCWSTR(name_w.as_ptr()));
                if format_id != 0 {
                    if let Ok(h_global) = GlobalAlloc(GHND, gif_bytes.len()) {
                        let p_mem = GlobalLock(h_global);
                        if !p_mem.is_null() {
                            std::ptr::copy_nonoverlapping(
                                gif_bytes.as_ptr(),
                                p_mem as *mut u8,
                                gif_bytes.len(),
                            );
                            let _ = GlobalUnlock(h_global);
                            let _ = SetClipboardData(
                                format_id,
                                Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
                            );
                        }
                    }
                }
            }
        }

        // 3. Set PNG format (if available) - many apps prefer PNG over DIB
        if let Some(png_bytes) = png_data {
            let png_format_names = ["PNG", "image/png"];

            for name in png_format_names {
                let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let format_id = RegisterClipboardFormatW(windows::core::PCWSTR(name_w.as_ptr()));
                if format_id != 0 {
                    if let Ok(h_global) = GlobalAlloc(GHND, png_bytes.len()) {
                        let p_mem = GlobalLock(h_global);
                        if !p_mem.is_null() {
                            std::ptr::copy_nonoverlapping(
                                png_bytes.as_ptr(),
                                p_mem as *mut u8,
                                png_bytes.len(),
                            );
                            let _ = GlobalUnlock(h_global);
                            let _ = SetClipboardData(
                                format_id,
                                Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
                            );
                        }
                    }
                }
            }
        }

        // 4. Set CF_DIB (universal fallback)
        let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
        let pixel_data_size = image.width * image.height * 4;
        let total_size = header_size + pixel_data_size;

        let h_global = GlobalAlloc(GHND, total_size).map_err(|e| e.to_string())?;
        let p_mem = GlobalLock(h_global);
        if p_mem.is_null() {
            return Err("GlobalLock failed".to_string());
        }

        let header = BITMAPINFOHEADER {
            bi_size: header_size as u32,
            bi_width: image.width as i32,
            bi_height: image.height as i32,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: 0,
            bi_size_image: pixel_data_size as u32,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        };

        std::ptr::copy_nonoverlapping(
            &header as *const _ as *const u8,
            p_mem as *mut u8,
            header_size,
        );

        let p_pixels = p_mem.add(header_size) as *mut u8;
        for y in 0..image.height {
            let src_y = image.height - 1 - y;
            let src_offset = src_y * image.width * 4;
            let dst_offset = y * image.width * 4;

            for x in 0..image.width {
                let s = src_offset + x * 4;
                let d = dst_offset + x * 4;
                *p_pixels.add(d) = image.bytes[s + 2];
                *p_pixels.add(d + 1) = image.bytes[s + 1];
                *p_pixels.add(d + 2) = image.bytes[s];
                *p_pixels.add(d + 3) = image.bytes[s + 3];
            }
        }

        let _ = GlobalUnlock(h_global);
        if SetClipboardData(
            CF_DIB,
            Some(windows::Win32::Foundation::HANDLE(h_global.0 as _)),
        )
        .is_err()
        {
            return Err("SetClipboardData (CF_DIB) failed".to_string());
        }

        Ok(hdrop_path.clone())
    })();

    let _ = CloseClipboard();
    result
}
