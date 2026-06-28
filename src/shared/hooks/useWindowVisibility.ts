import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const useWindowVisibility = () => {
  const isWindowVisibleRef = useRef(true);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    const syncVisible = () => {
      appWindow.isVisible()
        .then((visible) => {
          isWindowVisibleRef.current = visible;
        })
        .catch(() => {
          isWindowVisibleRef.current = false;
        });
    };

    const markHidden = () => {
      isWindowVisibleRef.current = false;
    };

    const unlistenBlur = appWindow.listen("tauri://blur", markHidden);
    const unlistenFocus = appWindow.listen("tauri://focus", syncVisible);
    const unlistenHide = appWindow.listen("tauri://hide", markHidden);
    const unlistenShow = appWindow.listen("tauri://show", syncVisible);
    const unlistenFocusChanged = appWindow.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        syncVisible();
      } else {
        markHidden();
      }
    });

    syncVisible();

    return () => {
      unlistenBlur.then(f => f());
      unlistenFocus.then(f => f());
      unlistenHide.then(f => f());
      unlistenShow.then(f => f());
      unlistenFocusChanged.then(f => f());
    };
  }, []);

  return isWindowVisibleRef;
};
