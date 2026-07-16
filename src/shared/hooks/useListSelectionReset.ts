import { useEffect, useRef } from "react";
import type { ClipboardEntry } from "../types";

interface UseListSelectionResetOptions {
  filteredHistory: ClipboardEntry[];
  selectedItemId: number | null;
  setSelectedIndex: (val: number) => void;
  setSelectedItemId: (val: number | null) => void;
}

export const useListSelectionReset = ({
  filteredHistory,
  selectedItemId,
  setSelectedIndex,
  setSelectedItemId
}: UseListSelectionResetOptions) => {
  const selectedItemIdRef = useRef(selectedItemId);
  selectedItemIdRef.current = selectedItemId;

  useEffect(() => {
    if (filteredHistory.length === 0) {
      setSelectedIndex(0);
      setSelectedItemId(null);
      return;
    }

    const prevId = selectedItemIdRef.current;
    if (prevId != null) {
      const nextIndex = filteredHistory.findIndex((item) => item.id === prevId);
      if (nextIndex >= 0) {
        setSelectedIndex(nextIndex);
        setSelectedItemId(prevId);
        return;
      }
    }

    const firstId = filteredHistory[0]?.id ?? null;
    setSelectedIndex(0);
    setSelectedItemId(firstId);
  }, [filteredHistory, setSelectedIndex, setSelectedItemId]);
};
