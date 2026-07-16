import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Dispatch, SetStateAction } from "react";
import type { ClipboardEntry } from "../types";

interface UsePinnedSortOptions {
  filteredHistory: ClipboardEntry[];
  history: ClipboardEntry[];
  setHistory: Dispatch<SetStateAction<ClipboardEntry[]>>;
}

const buildOrderedPinnedItems = (
  pinnedItems: ClipboardEntry[],
  pinnedOrderIds: number[]
): ClipboardEntry[] => {
  if (pinnedItems.length === 0) return [];

  const map = new Map<number, ClipboardEntry>();
  pinnedItems.forEach((item) => map.set(item.id, item));

  const ordered: ClipboardEntry[] = [];
  const seen = new Set<number>();

  pinnedOrderIds.forEach((id) => {
    const item = map.get(id);
    if (!item) return;
    ordered.push(item);
    seen.add(id);
  });

  pinnedItems.forEach((item) => {
    if (!seen.has(item.id)) {
      ordered.push(item);
    }
  });

  return ordered;
};

export const usePinnedSort = ({
  filteredHistory,
  history,
  setHistory
}: UsePinnedSortOptions) => {
  const { pinnedItems: rawPinnedItems, unpinnedItems } = useMemo(() => {
    return {
      pinnedItems: filteredHistory.filter((item) => item.is_pinned),
      unpinnedItems: filteredHistory.filter((item) => !item.is_pinned)
    };
  }, [filteredHistory]);

  const [pinnedOrderIds, setPinnedOrderIds] = useState<number[]>(() =>
    rawPinnedItems.map((item) => item.id)
  );
  const pinnedOrderRef = useRef<number[]>(rawPinnedItems.map((item) => item.id));
  const [isDraggingPinned, setIsDraggingPinned] = useState(false);

  useEffect(() => {
    if (isDraggingPinned) return;
    const next = rawPinnedItems.map((item) => item.id);
    setPinnedOrderIds(next);
    pinnedOrderRef.current = next;
  }, [rawPinnedItems, isDraggingPinned]);

  const pinnedItems = useMemo(
    () => buildOrderedPinnedItems(rawPinnedItems, pinnedOrderIds),
    [rawPinnedItems, pinnedOrderIds]
  );

  const navigableHistory = useMemo(
    () => [...pinnedItems, ...unpinnedItems],
    [pinnedItems, unpinnedItems]
  );

  const handlePinnedIdsReorder = useCallback((nextIds: number[]) => {
    setPinnedOrderIds(nextIds);
    pinnedOrderRef.current = nextIds;
  }, []);

  const handlePinnedDragStart = useCallback(() => {
    setIsDraggingPinned(true);
  }, []);

  const handlePinnedDragEnd = useCallback(() => {
    setIsDraggingPinned(false);
    const finalIds = pinnedOrderRef.current;
    const currentIds = rawPinnedItems.map((item) => item.id);
    if (
      finalIds.length === currentIds.length &&
      finalIds.every((id, idx) => id === currentIds[idx])
    ) {
      return;
    }

    const orderMap = new Map<number, number>();
    finalIds.forEach((id, index) => {
      orderMap.set(id, finalIds.length - index);
    });

    const nextHistory = history.map((item) => {
      const nextOrder = orderMap.get(item.id);
      if (nextOrder !== undefined) {
        return { ...item, pinned_order: nextOrder };
      }
      return item;
    });

    nextHistory.sort((a, b) => {
      if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
      if (a.is_pinned) {
        if ((a.pinned_order || 0) !== (b.pinned_order || 0)) {
          return (b.pinned_order || 0) - (a.pinned_order || 0);
        }
      }
      return b.timestamp - a.timestamp;
    });

    setHistory(nextHistory);

    const dbOrders = finalIds.map((id, index) => [id, finalIds.length - index]);
    invoke("update_pinned_order", { orders: dbOrders }).catch(console.error);
  }, [history, rawPinnedItems, setHistory]);

  return {
    pinnedItems,
    unpinnedItems,
    navigableHistory,
    pinnedOrderIds,
    isDraggingPinned,
    handlePinnedIdsReorder,
    handlePinnedDragStart,
    handlePinnedDragEnd
  };
};
