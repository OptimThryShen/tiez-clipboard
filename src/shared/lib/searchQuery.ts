export type ParsedSearchQuery = {
  raw: string;
  term: string;
  tagOnly: boolean;
  noteOnly: boolean;
};

/** Parse `tag:` / `note:` search command prefixes. */
export function parseSearchQuery(raw: string): ParsedSearchQuery {
  const trimmed = raw.trim();
  const lower = trimmed.toLowerCase();

  if (lower.startsWith("tag:")) {
    return {
      raw: trimmed,
      term: trimmed.slice(4).trim().toLowerCase(),
      tagOnly: true,
      noteOnly: false
    };
  }

  if (lower.startsWith("note:")) {
    return {
      raw: trimmed,
      term: trimmed.slice(5).trim().toLowerCase(),
      tagOnly: false,
      noteOnly: true
    };
  }

  return {
    raw: trimmed,
    term: trimmed.toLowerCase(),
    tagOnly: false,
    noteOnly: false
  };
}
