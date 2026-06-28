import { useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriRuntime } from "../lib/tauriRuntime";

interface UseNavigationSyncOptions {
  showSettings: boolean;
  showTagManager: boolean;
  chatMode: boolean;
  showEmojiPanel: boolean;
}

export const useNavigationSync = ({
  showSettings,
  showTagManager,
  chatMode,
  showEmojiPanel
}: UseNavigationSyncOptions) => {
  const syncNavigationEnabled = useCallback(() => {
    if (!isTauriRuntime()) return;

    const shouldDisableNavigation = showSettings || showTagManager || chatMode || showEmojiPanel;
    if (shouldDisableNavigation) {
      invoke("set_navigation_enabled", { enabled: false }).catch(console.error);
      return;
    }

    // Clipboard list mode: only enable keyboard/mouse routing when the window is visible.
    getCurrentWindow()
      .isVisible()
      .then((visible) => {
        invoke("set_navigation_enabled", { enabled: visible }).catch(console.error);
      })
      .catch(() => {
        invoke("set_navigation_enabled", { enabled: false }).catch(console.error);
      });
  }, [showSettings, showTagManager, chatMode, showEmojiPanel]);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    syncNavigationEnabled();

    const appWindow = getCurrentWindow();
    const unlistenShow = appWindow.listen("tauri://show", syncNavigationEnabled);
    const unlistenHide = appWindow.listen("tauri://hide", () => {
      invoke("set_navigation_enabled", { enabled: false }).catch(console.error);
    });

    return () => {
      unlistenShow.then((unlisten) => unlisten()).catch(() => {});
      unlistenHide.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [syncNavigationEnabled]);
};
