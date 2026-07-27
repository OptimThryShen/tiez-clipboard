import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { toTauriLocalImageSrc } from "../lib/localImageSrc";
import {
  isHtmlishTagText,
  isOfficeStyleDefinitionText,
  stripOfficePreviewNoise
} from "../lib/repairHtmlFragment";

const pickFirstSrcFromSrcset = (srcset?: string | null): string | null => {
  if (!srcset) return null;
  const first = srcset
    .split(",")
    .map((part) => part.trim())
    .find(Boolean);
  if (!first) return null;
  const url = first.split(/\s+/)[0]?.trim();
  return url || null;
};

/** Drop authoring font sizes so list/hover preview follows app typography. */
const stripFontSizeFromCssText = (cssText: string): string => {
  return cssText
    .replace(/font-size\s*:\s*[^;]+;?/gi, "")
    .replace(/;\s*;/g, ";")
    .replace(/^;+|;+$/g, "")
    .trim();
};

/** Preview colors inherit the active app theme; source HTML remains untouched. */
const stripPreviewColorFromCssText = (cssText: string): string => {
  return cssText
    .replace(/(^|[;{])\s*(?:color|background-color)\s*:\s*[^;}]+;?/gi, "$1")
    .replace(
      /(^|[;{])\s*background\s*:\s*([^;}]+);?/gi,
      (full, prefix: string, value: string) =>
        /(?:url|gradient)\s*\(/i.test(value) ? full : prefix
    )
    .replace(/;\s*;/g, ";")
    .trim();
};

const stripUnsafeCssText = (cssText: string): string => {
  return cssText
    .replace(/@import\s+[^;]+;?/gi, "")
    .replace(/expression\s*\([^)]*\)/gi, "")
    .replace(/(?:^|[;{])\s*(?:position|z-index|inset|top|right|bottom|left)\s*:\s*[^;}]+;?/gi, "$1")
    .replace(
      /url\s*\(\s*(['"]?)(.*?)\1\s*\)/gi,
      (full, _quote: string, rawUrl: string) => {
        const url = rawUrl.trim();
        return /^(?:data:image\/|https?:\/\/asset\.localhost\/|asset:)/i.test(url)
          ? full
          : "none";
      }
    )
    .replace(/;\s*;/g, ";")
    .trim();
};

const resolveImgSource = (el: Element): string | null => {
  const src = el.getAttribute("src")?.trim() || "";
  const lazyAttrs = [
    "data-src",
    "data-original",
    "data-lazy-src",
    "data-actualsrc",
    "data-url"
  ];
  const lazySrc =
    lazyAttrs
      .map((name) => el.getAttribute(name)?.trim() || "")
      .find(Boolean) || "";
  const srcsetPrimary = pickFirstSrcFromSrcset(el.getAttribute("srcset"));
  const lazySrcsetPrimary = pickFirstSrcFromSrcset(el.getAttribute("data-srcset"));

  const isMissingOrPlaceholder =
    !src ||
    src === "about:blank" ||
    src === "#" ||
    src.toLowerCase().startsWith("blob:");

  if (isMissingOrPlaceholder) {
    return lazySrc || srcsetPrimary || lazySrcsetPrimary || null;
  }
  return src;
};

const sanitizeHTML = (html: string, preview?: boolean) => {
  const parser = new DOMParser();

  const stripLeadingPreviewNoise = (container: HTMLElement) => {
    if (!container.querySelector("table, p, div, img, ul, ol, blockquote, pre")) return;

    for (const node of Array.from(container.childNodes)) {
      if (node.nodeType === Node.TEXT_NODE) {
        const text = node.textContent?.trim() || "";
        if (!text) {
          container.removeChild(node);
          continue;
        }
        if (isHtmlishTagText(text) || isOfficeStyleDefinitionText(text)) {
          container.removeChild(node);
          continue;
        }
      }

      if (node.nodeType === Node.ELEMENT_NODE) {
        const element = node as Element;
        const tagName = element.tagName.toLowerCase();
        if (tagName === "style" && isOfficeStyleDefinitionText(element.textContent || "")) {
          container.removeChild(node);
          continue;
        }
        if (tagName === "meta" || tagName === "link" || tagName === "xml") {
          container.removeChild(node);
          continue;
        }
      }

      break;
    }
  };

  // Heuristic: If it contains table elements but no <table> tag, wrap it.
  let processedHtml = stripOfficePreviewNoise(html);
  if ((processedHtml.includes("<tr") || processedHtml.includes("<td") || processedHtml.includes("<col"))
    && !processedHtml.toLowerCase().includes("<table")) {
    processedHtml = `<table style="border-collapse: collapse; min-width: 100%;">${processedHtml}</table>`;
  }

  const doc = parser.parseFromString(processedHtml, "text/html");

  // Keep formatting styles, but remove active/embedded content. Author styles are
  // rendered inside a ShadowRoot below, so their selectors cannot leak into TieZ.
  doc
    .querySelectorAll(
      "script, iframe, frame, object, embed, form, input, button, textarea, select, option, base, template, svg, math"
    )
    .forEach(el => el.remove());
  doc.querySelectorAll("style").forEach((style) => {
    if (isOfficeStyleDefinitionText(style.textContent || "")) {
      style.remove();
      return;
    }
    if (preview && style.textContent) {
      style.textContent = stripPreviewColorFromCssText(
        stripFontSizeFromCssText(style.textContent)
      );
    }
    if (style.textContent) {
      style.textContent = stripUnsafeCssText(style.textContent);
    }
  });
  doc.querySelectorAll("meta, link, xml").forEach((el) => el.remove());

  // Truncate tables for preview to save performance
  if (preview) {
    doc.querySelectorAll("table").forEach(table => {
      const rows = table.querySelectorAll("tr");
      if (rows.length > 5) {
        // Keep first 3 rows
        for (let i = 4; i < rows.length; i++) {
          rows[i].remove();
        }
        // Add a "..." indicator
        const moreRow = doc.createElement("tr");
        const moreCell = doc.createElement("td");
        moreCell.colSpan = 10;
        moreCell.style.textAlign = "center";
        moreCell.style.fontSize = "10px";
        moreCell.style.opacity = "0.5";
        moreCell.innerText = "... content truncated for preview ...";
        moreRow.appendChild(moreCell);
        table.appendChild(moreRow);
      }
    });
  }

  // Move styles from head to body to ensure they are included in the final innerHTML
  doc.head.querySelectorAll("style").forEach(style => {
    doc.body.prepend(style);
  });
  stripLeadingPreviewNoise(doc.body);

  const all = doc.querySelectorAll("*");
  all.forEach(el => {
    // Basic sanitization of on* attributes and javascript: links
    [...el.attributes].forEach(attr => {
      const name = attr.name.toLowerCase();
      const value = attr.value.trim().toLowerCase();
      if (name.startsWith("on")) {
        el.removeAttribute(attr.name);
      }
      if ((name === "href" || name === "src") && value.startsWith("javascript:")) {
        el.removeAttribute(attr.name);
      }
      if (name === "style") {
        let cleanedStyle = attr.value
          .replace(/(?:^|;)\s*(?:transform|writing-mode|rotate|scale)\s*:[^;]*/gi, "")
          .trim()
          .replace(/^;+|;+$/g, "");
        cleanedStyle = stripUnsafeCssText(cleanedStyle);
        if (preview) {
          cleanedStyle = stripPreviewColorFromCssText(cleanedStyle);
          cleanedStyle = stripFontSizeFromCssText(cleanedStyle);
        }
        if (cleanedStyle) {
          el.setAttribute("style", cleanedStyle);
        } else {
          el.removeAttribute("style");
        }
      }
      // Presentational <font size="..."> from older HTML / Office paste
      if (preview && el.tagName.toLowerCase() === "font" && name === "size") {
        el.removeAttribute(attr.name);
      }
      if (preview && (name === "color" || name === "bgcolor")) {
        el.removeAttribute(attr.name);
      }
    });

    // Handle local file images (including encoded file:// paths)
    if (el.tagName.toLowerCase() === 'img') {
      const resolvedSrc = resolveImgSource(el);
      if (resolvedSrc) {
        if (el.getAttribute("src") !== resolvedSrc) {
          el.setAttribute("src", resolvedSrc);
        }
        const normalizedSrc = resolvedSrc.startsWith("//") ? `https:${resolvedSrc}` : resolvedSrc;
        if (normalizedSrc !== resolvedSrc) {
          el.setAttribute("src", normalizedSrc);
        }

        const mappedSrc = toTauriLocalImageSrc(normalizedSrc);
        if (mappedSrc) {
          el.setAttribute('src', mappedSrc);
          (el as HTMLElement).style.maxWidth = '100%';
          (el as HTMLElement).style.height = 'auto';
        } else if (/^https?:\/\//i.test(normalizedSrc)) {
          // Some hosts reject unknown referrers; no-referrer is more broadly accepted.
          el.setAttribute("referrerpolicy", "no-referrer");
        }
      }
    }
  });

  const bodyClone = doc.body.cloneNode(true) as HTMLElement;
  bodyClone.querySelectorAll("style, script").forEach(el => el.remove());
  const hasRenderableText = (bodyClone.textContent || "").trim().length > 0;
  const hasRenderableElement = !!bodyClone.querySelector("*");

  return { html: doc.body.innerHTML, hasRenderable: hasRenderableText || hasRenderableElement };
};

type HtmlContentProps = {
  htmlContent: string;
  fallbackText?: string;
  className?: string;
  style?: React.CSSProperties;
  preview?: boolean;
};

const SHADOW_BASE_STYLE = `
  :host {
    display: block;
    color: var(--text-primary);
    font-family: var(--font-main);
    font-size: var(--clipboard-item-font-size);
    line-height: 1.4;
  }
  .tiez-rich-root { color: inherit; font: inherit; line-height: inherit; min-width: 0; }
  .tiez-rich-root *, .tiez-rich-root *::before, .tiez-rich-root *::after {
    box-sizing: border-box;
    max-width: 100%;
  }
  .tiez-rich-root img, .tiez-rich-root video { max-width: 100%; height: auto; }
  .tiez-rich-root table { border-collapse: collapse; max-width: 100%; }
`;

const SHADOW_PREVIEW_STYLE = `
  .tiez-rich-root, .tiez-rich-root *:not(style) {
    font-size: inherit !important;
    line-height: inherit;
    color: var(--text-primary) !important;
    background-color: transparent !important;
  }
  .tiez-rich-root table { width: 100%; margin: 4px 0; }
  .tiez-rich-root td, .tiez-rich-root th {
    border: 1px solid var(--border-dark);
    padding: 2px 4px;
    background-color: var(--bg-element) !important;
    color: var(--text-primary) !important;
  }
  .tiez-rich-root img { max-height: 80px; object-fit: contain; }
  .tiez-rich-root p { margin: 0; padding: 0; }
  .tiez-rich-root h1, .tiez-rich-root h2, .tiez-rich-root h3, .tiez-rich-root h4 {
    font-size: 1em !important;
    font-weight: bold;
    margin: 2px 0;
  }
  .tiez-rich-root ul, .tiez-rich-root ol { padding-left: 16px; margin: 0; }
  .tiez-rich-root a { color: var(--accent-color) !important; text-decoration: underline; }
`;

type Rgba = { r: number; g: number; b: number; a: number };

const parseCssColor = (value: string): Rgba | null => {
  const match = value.match(
    /^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:\s*[,/]\s*([\d.]+))?\s*\)$/i
  );
  if (!match) return null;
  return {
    r: Number(match[1]),
    g: Number(match[2]),
    b: Number(match[3]),
    a: match[4] == null ? 1 : Number(match[4])
  };
};

const compositeColor = (foreground: Rgba, background: Rgba): Rgba => {
  const alpha = foreground.a + background.a * (1 - foreground.a);
  if (alpha <= 0) return { r: 0, g: 0, b: 0, a: 0 };
  return {
    r: (foreground.r * foreground.a + background.r * background.a * (1 - foreground.a)) / alpha,
    g: (foreground.g * foreground.a + background.g * background.a * (1 - foreground.a)) / alpha,
    b: (foreground.b * foreground.a + background.b * background.a * (1 - foreground.a)) / alpha,
    a: alpha
  };
};

const relativeLuminance = ({ r, g, b }: Rgba) => {
  const linear = [r, g, b].map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
};

const contrastRatio = (left: Rgba, right: Rgba) => {
  const light = Math.max(relativeLuminance(left), relativeLuminance(right));
  const dark = Math.min(relativeLuminance(left), relativeLuminance(right));
  return (light + 0.05) / (dark + 0.05);
};

const effectiveBackground = (element: Element, host: HTMLElement): Rgba => {
  let background: Rgba = { r: 0, g: 0, b: 0, a: 0 };
  let current: Element | null = element;

  while (current) {
    const parsed = parseCssColor(getComputedStyle(current).backgroundColor);
    if (parsed && parsed.a > 0) {
      background = compositeColor(background, parsed);
      if (background.a >= 0.99) return background;
    }
    current = current.parentElement;
  }

  current = host;
  while (current) {
    const parsed = parseCssColor(getComputedStyle(current).backgroundColor);
    if (parsed && parsed.a > 0) {
      background = compositeColor(background, parsed);
      if (background.a >= 0.99) return background;
    }
    current = current.parentElement;
  }

  const fallback = parseCssColor(
    getComputedStyle(host).getPropertyValue("--bg-element").trim()
  );
  return fallback ?? { r: 255, g: 255, b: 255, a: 1 };
};

const repairUnreadableSourceColors = (root: ShadowRoot, host: HTMLElement) => {
  root.querySelectorAll<HTMLElement>(".tiez-rich-root *").forEach((element) => {
    if (!(element.textContent || "").trim()) return;
    const foreground = parseCssColor(getComputedStyle(element).color);
    if (!foreground) return;
    const background = effectiveBackground(element, host);
    const opaqueForeground = compositeColor(foreground, background);
    if (contrastRatio(opaqueForeground, background) < 2.25) {
      element.style.setProperty("color", "var(--text-primary)", "important");
    }
  });
};

const HtmlContent = ({ htmlContent, fallbackText, className, style, preview }: HtmlContentProps) => {
  const contentRef = useRef<HTMLDivElement | null>(null);
  const processedRef = useRef<{ htmlContent: string; preview?: boolean; fallbackText?: string } | null>(null);
  const [isVisible, setIsVisible] = useState(!!preview);
  const previewPlaceholderMinHeight = (() => {
    const maxH = style?.maxHeight;
    if (typeof maxH === "number" && Number.isFinite(maxH)) return `${maxH}px`;
    if (typeof maxH === "string" && maxH.trim()) return maxH;
    return "40px";
  })();

  useEffect(() => {
    if (preview) return;
    if (!contentRef.current) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" } // Load slightly before coming into view
    );

    observer.observe(contentRef.current);
    return () => observer.disconnect();
  }, [preview]);

  useLayoutEffect(() => {
    if (!isVisible || !contentRef.current) return;
    const prev = processedRef.current;
    if (prev && prev.htmlContent === htmlContent && prev.preview === preview && prev.fallbackText === fallbackText) {
      return;
    }
    processedRef.current = { htmlContent, preview, fallbackText };
    const { html: cleanHTML, hasRenderable } = sanitizeHTML(htmlContent, preview);
    const host = contentRef.current;
    const shadowRoot = host.shadowRoot ?? host.attachShadow({ mode: "open" });
    const renderedContent = !hasRenderable && fallbackText
      ? `<div class="tiez-rich-root"></div>`
      : `<div class="tiez-rich-root">${cleanHTML}</div>`;
    shadowRoot.innerHTML = `<style>${SHADOW_BASE_STYLE}${preview ? SHADOW_PREVIEW_STYLE : ""}</style>${renderedContent}`;

    if (!hasRenderable && fallbackText) {
      const fallbackRoot = shadowRoot.querySelector(".tiez-rich-root");
      if (fallbackRoot) fallbackRoot.textContent = fallbackText;
    } else if (!preview) {
      requestAnimationFrame(() => repairUnreadableSourceColors(shadowRoot, host));
    }
  }, [htmlContent, fallbackText, isVisible, preview]);

  return (
    <div
      ref={contentRef}
      className={className}
      style={{
        minHeight: isVisible ? undefined : (preview ? previewPlaceholderMinHeight : '100px'),
        ...style
      }}
    />
  );
};

export default HtmlContent;
