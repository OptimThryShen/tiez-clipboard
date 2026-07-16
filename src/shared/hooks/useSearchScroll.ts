import { useCallback, useEffect, useRef, type RefObject } from "react";

type UseSearchScrollOptions = {
  containerRef: RefObject<HTMLElement | null>;
  showSearchBox: boolean;
  setShowSearchBox: (val: boolean) => void;
  search: string;
  showSettings: boolean;
  showTagManager: boolean;
  appSettings: Record<string, string>;
  dismissSearchTagFilter?: () => void;
};

const SHOW_THRESHOLD = 16;
const WHEEL_DELTA_MIN = 6;
/** Treat near-top as top (Virtuoso may report small non-zero offsets). */
const AT_TOP_EPSILON = 12;
/** Brief pause after landing at top to avoid momentum overscroll triggering reveal */
const TOP_SETTLE_MS = 72;

export const useSearchScroll = ({
  containerRef,
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
  const wheelAttachedRef = useRef(false);
  const wheelListenerRef = useRef<((e: WheelEvent) => void) | null>(null);

  const showSearchBoxRef = useRef(showSearchBox);
  const searchRef = useRef(search);
  const searchPinnedRef = useRef(appSettings["app.show_search_box"] === "true");
  const showSettingsRef = useRef(showSettings);
  const showTagManagerRef = useRef(showTagManager);
  const setShowSearchBoxRef = useRef(setShowSearchBox);
  const dismissRef = useRef(dismissSearchTagFilter);

  showSearchBoxRef.current = showSearchBox;
  searchRef.current = search;
  searchPinnedRef.current = appSettings["app.show_search_box"] === "true";
  showSettingsRef.current = showSettings;
  showTagManagerRef.current = showTagManager;
  setShowSearchBoxRef.current = setShowSearchBox;
  dismissRef.current = dismissSearchTagFilter;

  const shouldCaptureWheel = useCallback(() => {
    if (searchPinnedRef.current || showSettingsRef.current || showTagManagerRef.current) {
      return false;
    }
    return showSearchBoxRef.current || listScrollTopRef.current <= AT_TOP_EPSILON;
  }, []);

  const syncWheelListener = useCallback(() => {
    const el = containerRef.current;
    const listener = wheelListenerRef.current;
    if (!el || !listener) return;

    const want = shouldCaptureWheel();
    if (want && !wheelAttachedRef.current) {
      el.addEventListener("wheel", listener, { capture: true, passive: false });
      wheelAttachedRef.current = true;
    } else if (!want && wheelAttachedRef.current) {
      el.removeEventListener("wheel", listener, { capture: true });
      wheelAttachedRef.current = false;
      scrollTriggerRef.current = 0;
    }
  }, [containerRef, shouldCaptureWheel]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const isAtTop = () => listScrollTopRef.current <= AT_TOP_EPSILON;

    const onWheel = (e: WheelEvent) => {
      const deltaY = e.deltaY;
      if (Math.abs(deltaY) <= WHEEL_DELTA_MIN) return;

      const atTop = isAtTop();
      const scrollingDown = deltaY > 0;
      const scrollingUp = deltaY < 0;
      const searchOpen = showSearchBoxRef.current;
      const searchEmpty = searchRef.current.length === 0;

      if (scrollingDown && searchOpen && searchEmpty) {
        dismissRef.current?.();
        showSearchBoxRef.current = false;
        setShowSearchBoxRef.current(false);
        scrollTriggerRef.current = 0;
        if (atTop) {
          e.preventDefault();
          e.stopPropagation();
        }
        syncWheelListener();
        return;
      }

      if (scrollingUp && atTop && !searchOpen) {
        if (Date.now() - topReachedTimeRef.current > TOP_SETTLE_MS) {
          scrollTriggerRef.current += Math.abs(deltaY);
          if (scrollTriggerRef.current >= SHOW_THRESHOLD) {
            e.preventDefault();
            e.stopPropagation();
            dismissRef.current?.();
            showSearchBoxRef.current = true;
            setShowSearchBoxRef.current(true);
            scrollTriggerRef.current = 0;
            syncWheelListener();
            return;
          }
          e.preventDefault();
          e.stopPropagation();
        } else {
          scrollTriggerRef.current = 0;
        }
      } else if (!scrollingUp || !atTop) {
        scrollTriggerRef.current = 0;
      }
    };

    wheelListenerRef.current = onWheel;
    syncWheelListener();

    return () => {
      if (wheelAttachedRef.current && wheelListenerRef.current) {
        el.removeEventListener("wheel", wheelListenerRef.current, { capture: true });
        wheelAttachedRef.current = false;
      }
      wheelListenerRef.current = null;
    };
  }, [containerRef, syncWheelListener]);

  useEffect(() => {
    syncWheelListener();
  }, [showSearchBox, showSettings, showTagManager, appSettings, syncWheelListener]);

  const handleListScroll = useCallback(
    (offset: number) => {
      if (offset === listScrollTopRef.current) return;

      const wasAtTop = listScrollTopRef.current <= AT_TOP_EPSILON;
      if (offset <= AT_TOP_EPSILON && listScrollTopRef.current > AT_TOP_EPSILON) {
        topReachedTimeRef.current = Date.now();
      }
      listScrollTopRef.current = offset;

      if (wasAtTop !== offset <= AT_TOP_EPSILON) {
        syncWheelListener();
      }
    },
    [syncWheelListener]
  );

  return {
    handleListScroll
  };
};
