const MISSING_LEADING_TAG_RE =
  /^(table|tbody|thead|tfoot|tr|td|th|colgroup|col|div|span|p|ul|ol|li|blockquote|pre|h[1-6]|meta|style|img|a)\b[^>]*>/i;
const OFFICE_STYLE_SIGNAL_RE =
  /(?:\/\*\s*style definitions\s*\*\/|mso-style-name|mso-style-noshow|mso-style-priority|mso-padding-alt|mso-para-margin|table\.mso|mso-|microsoftinternetexplorer\d*|documentnotspecified|wps office|office word|msonormal|mso normal|normal\s+\d+\s+false)/i;
const RENDERABLE_CONTENT_TAG_RE =
  /<(table|p|div|span|img|a|ul|ol|li|blockquote|pre|h[1-6])\b/i;
const OFFICE_STYLE_BLOCK_RE = /<style\b[\s\S]*?<\/style>/gi;
const OFFICE_XML_BLOCK_RE = /<xml\b[\s\S]*?<\/xml>/gi;
const CONDITIONAL_COMMENT_RE = /<!--[\s\S]*?-->/gi;
const BODY_RE = /<body\b[^>]*>([\s\S]*?)<\/body\s*>/i;
const HEAD_RE = /<head\b[\s\S]*?<\/head\s*>/gi;

export const isHtmlishTagText = (text: string): boolean => {
  return MISSING_LEADING_TAG_RE.test((text || "").trim());
};

export const hasRenderableCssRules = (text: string): boolean => {
  if (!text || !text.includes("{")) return false;
  return /(?:^|[;{\s])(?:color|font|background|text-decoration|line-height|letter-spacing|border|mso-font|mso-highlight)\s*:/im.test(
    text
  );
};

export const isOfficeStyleDefinitionText = (text: string): boolean => {
  const normalized = (text || "").replace(/\s+/g, " ").trim();
  if (normalized.length <= 24 || !OFFICE_STYLE_SIGNAL_RE.test(normalized)) {
    return false;
  }
  // WPS/Word style blocks often include mso-* metadata and real CSS rules together.
  return !hasRenderableCssRules(normalized);
};

export const repairHtmlFragment = (html: string): string => {
  const trimmed = (html || "").trim();
  if (!trimmed || trimmed.startsWith("<")) {
    return trimmed;
  }

  if (isHtmlishTagText(trimmed)) {
    return `<${trimmed}`;
  }

  return trimmed;
};

const RICH_NAMED_FORMATS_PREFIX = "<!--TIEZ_RICH_FORMATS:";
const RICH_NAMED_FORMATS_SUFFIX = "-->";

export const stripRichStorageMarkers = (html: string): string => {
  let processed = (html || "").trim();
  if (!processed) return processed;

  const imageStart = processed.lastIndexOf("<!--TIEZ_RICH_IMAGE:");
  if (imageStart >= 0) {
    const markerStart = imageStart + "<!--TIEZ_RICH_IMAGE:".length;
    const endRel = processed.slice(markerStart).indexOf("-->");
    if (endRel >= 0) {
      const markerEnd = markerStart + endRel;
      processed = `${processed.slice(0, imageStart)}${processed.slice(markerEnd + 3)}`.trim();
    }
  }

  const formatsStart = processed.lastIndexOf(RICH_NAMED_FORMATS_PREFIX);
  if (formatsStart >= 0) {
    const markerStart = formatsStart + RICH_NAMED_FORMATS_PREFIX.length;
    const endRel = processed.slice(markerStart).indexOf(RICH_NAMED_FORMATS_SUFFIX);
    if (endRel >= 0) {
      const markerEnd = markerStart + endRel;
      processed = `${processed.slice(0, formatsStart)}${processed.slice(markerEnd + RICH_NAMED_FORMATS_SUFFIX.length)}`.trim();
    }
  }

  return processed;
};

export const extractRenderableHtmlFragment = (html: string): string => {
  const repaired = repairHtmlFragment(html || "");
  const trimmed = repaired.trim();
  if (!trimmed) {
    return trimmed;
  }

  const startMarker = "<!--StartFragment-->";
  const endMarker = "<!--EndFragment-->";
  const startIndex = trimmed.indexOf(startMarker);
  if (startIndex >= 0) {
    const contentStart = startIndex + startMarker.length;
    const endIndex = trimmed.indexOf(endMarker, contentStart);
    if (endIndex > contentStart) {
      return trimmed.slice(contentStart, endIndex).trim();
    }
  }

  const bodyMatch = trimmed.match(BODY_RE);
  if (bodyMatch?.[1]) {
    return bodyMatch[1].trim();
  }

  return trimmed.replace(HEAD_RE, " ").trim();
};

export const stripOfficePreviewNoise = (html: string): string => {
  let processed = extractRenderableHtmlFragment(html || "");
  if (!processed) {
    return processed;
  }

  processed = processed.replace(OFFICE_XML_BLOCK_RE, (block) =>
    isOfficeStyleDefinitionText(block) ? " " : block
  );
  processed = processed.replace(OFFICE_STYLE_BLOCK_RE, (block) =>
    isOfficeStyleDefinitionText(block) ? " " : block
  );
  processed = processed.replace(CONDITIONAL_COMMENT_RE, (block) =>
    isOfficeStyleDefinitionText(block) ? " " : block
  );

  const match = RENDERABLE_CONTENT_TAG_RE.exec(processed);
  if (match && match.index > 0) {
    const prefix = processed.slice(0, match.index);
    if (isOfficeStyleDefinitionText(prefix)) {
      processed = processed.slice(match.index);
    }
  }

  return processed.trim();
};
