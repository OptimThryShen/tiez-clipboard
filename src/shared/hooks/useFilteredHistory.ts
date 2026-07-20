import { useMemo } from "react";
import type { ClipboardEntry } from "../types";
import { parseSearchQuery } from "../lib/searchQuery";
import { entryMatchesSearch } from "../lib/searchMatch";

interface UseFilteredHistoryOptions {
  history: ClipboardEntry[];
  search: string;
  typeFilter: string | null;
}

export const useFilteredHistory = ({
  history,
  search,
  typeFilter
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

    return filtered.sort((a, b) => {
      if (a.is_pinned !== b.is_pinned) {
        return a.is_pinned ? -1 : 1;
      }
      if (a.is_pinned) {
        if ((a.pinned_order || 0) !== (b.pinned_order || 0)) {
          return (b.pinned_order || 0) - (a.pinned_order || 0);
        }
        return b.timestamp - a.timestamp;
      }
      return b.timestamp - a.timestamp;
    });
  }, [history, search, typeFilter]);
};
