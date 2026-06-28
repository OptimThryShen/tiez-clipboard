import { useCallback, useRef } from "react";
import type { WheelEvent as ReactWheelEvent } from "react";

type UseSearchScrollOptions = {
  showSearchBox: boolean;
  setShowSearchBox: (val: boolean) => void;
  search: string;
  showSettings: boolean;
  showTagManager: boolean;
  appSettings: Record<string, string>;
  dismissSearchTagFilter?: () => void;
};

export const useSearchScroll = ({
  showSearchBox,
  setShowSearchBox,
  search,
  showSettings,
  showTagManager,
  appSettings,
  dismissSearchTagFilter
}: UseSearchScrollOptions) => {
  const scrollTriggerRef = useRef(0);
  const listScrollTopRef = useRef(0);
  const topReachedTimeRef = useRef(0);

  const handleListScroll = useCallback((offset: number) => {
    if (offset === 0 && listScrollTopRef.current > 0) {
      topReachedTimeRef.current = Date.now();
    }
    listScrollTopRef.current = offset;
  }, []);

  const handleMainWheel = useCallback(
    (e: ReactWheelEvent<HTMLElement>) => {
      if (showSettings || showTagManager) return;

      if (
        e.deltaY < -5 &&
        (listScrollTopRef.current === 0 || isNaN(listScrollTopRef.current))
      ) {
        if (Date.now() - topReachedTimeRef.current > 250) {
          if (!showSearchBox) {
            scrollTriggerRef.current += Math.abs(e.deltaY);
            if (scrollTriggerRef.current > 45) {
              dismissSearchTagFilter?.();
              setShowSearchBox(true);
              scrollTriggerRef.current = 0;
            }
          }
        } else {
          scrollTriggerRef.current = 0;
        }
      } else {
        scrollTriggerRef.current = 0;
      }

      if (e.deltaY > 10) {
        dismissSearchTagFilter?.();
      }

      if (
        e.deltaY > 10 &&
        showSearchBox &&
        search.trim() === "" &&
        appSettings["app.show_search_box"] !== "true"
      ) {
        // See App.tsx note: do not persist setting when hiding temporary search.
        dismissSearchTagFilter?.();
        setShowSearchBox(false);
      }
    },
    [
      showSettings,
      showTagManager,
      showSearchBox,
      search,
      appSettings,
      setShowSearchBox,
      dismissSearchTagFilter
    ]
  );

  return {
    handleListScroll,
    handleMainWheel
  };
};
