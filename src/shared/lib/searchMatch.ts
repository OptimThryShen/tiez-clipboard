import type { ClipboardEntry } from "../types";

const TEXT_TYPES = new Set(["text", "code", "url", "rich_text"]);

export function entryMatchesSearch(
  item: ClipboardEntry,
  search: string,
  tagOnly: boolean
): boolean {
  const term = search.trim().toLowerCase();
  if (!term) {
    return true;
  }

  if (tagOnly) {
    return item.tags?.some((tag) => tag.toLowerCase() === term) ?? false;
  }

  if (item.tags?.some((tag) => tag.toLowerCase().includes(term))) {
    return true;
  }

  if (item.source_app?.toLowerCase().includes(term)) {
    return true;
  }

  if (item.note?.toLowerCase().includes(term)) {
    return true;
  }

  if (TEXT_TYPES.has(item.content_type)) {
    const preview = item.preview?.toLowerCase() ?? "";
    if (preview.includes(term)) {
      return true;
    }
    return item.content?.toLowerCase().includes(term) ?? false;
  }

  return false;
}
