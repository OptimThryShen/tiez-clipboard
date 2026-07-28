export interface ClipboardEntry {
  id: number;
  content_type: string;
  content: string;
  html_content?: string;
  source_app: string;
  source_app_path?: string;
  /** Legacy-compatible ordering timestamp; mirrors sort_at. */
  timestamp: number;
  /** Immutable first-capture/import time. */
  created_at?: number;
  /** Most recent paste/open time. */
  last_used_at?: number;
  /** Mutable list ordering key. */
  sort_at?: number;
  preview: string;
  is_pinned: boolean;
  tags: string[];
  /** Free-text remark (not a category tag) */
  note?: string;
  isInputting?: boolean;
  questionCount?: number;
  use_count?: number;
  is_external?: boolean;
  pinned_order?: number;
  file_preview_exists?: boolean;
}
