import { useMemo } from "react";
import type { ClipboardEntry } from "../types";
import type { ClipboardSortMode } from "../../features/app/types";
import { parseSearchQuery } from "../lib/searchQuery";
import { entryMatchesSearch } from "../lib/searchMatch";

interface UseFilteredHistoryOptions {
  history: ClipboardEntry[];
  search: string;
  typeFilter: string | null;
  sortMode: ClipboardSortMode;
}

const activityAt = (item: ClipboardEntry) => item.sort_at || item.timestamp;

export const compareClipboardEntries = (
  a: ClipboardEntry,
  b: ClipboardEntry,
  sortMode: ClipboardSortMode
) => {
  if (a.is_pinned !== b.is_pinned) {
    return a.is_pinned ? -1 : 1;
  }
  if (a.is_pinned) {
    return (
      (b.pinned_order || 0) - (a.pinned_order || 0) ||
      activityAt(b) - activityAt(a) ||
      b.id - a.id
    );
  }

  let selectedDifference = 0;
  switch (sortMode) {
    case "created":
      selectedDifference =
        (b.created_at || b.timestamp) - (a.created_at || a.timestamp);
      break;
    case "last_used":
      selectedDifference = (b.last_used_at || 0) - (a.last_used_at || 0);
      break;
    case "usage":
      selectedDifference = (b.use_count || 0) - (a.use_count || 0);
      break;
    case "activity":
    default:
      selectedDifference = activityAt(b) - activityAt(a);
      break;
  }

  return selectedDifference || activityAt(b) - activityAt(a) || b.id - a.id;
};

export const useFilteredHistory = ({
  history,
  search,
  typeFilter,
  sortMode
}: UseFilteredHistoryOptions) => {
  return useMemo(() => {
    const parsed = parseSearchQuery(search);

    const filtered = history.filter((item) => {
      if (typeFilter && item.content_type !== typeFilter) {
        return false;
      }

      if (!parsed.raw) return true;

      return entryMatchesSearch(item, parsed);
    });

    return filtered.sort((a, b) => compareClipboardEntries(a, b, sortMode));
  }, [history, search, typeFilter, sortMode]);
};
