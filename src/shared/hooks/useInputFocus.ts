import { useEffect, useRef, type MutableRefObject } from "react";
import { invoke } from "@tauri-apps/api/core";
import { restoreLastFocus } from "../lib/focus";
import { isMacPlatform } from "../lib/platform";
import { isTauriRuntime } from "../lib/tauriRuntime";

type FocusState = "normal" | "clipboard";

type UseInputFocusOptions = {
  enableDelay?: number;
  restoreDelay?: number;
};

const isEditableElement = (el: Element | null) => {
  if (!el) return false;
  const tagName = el.tagName;
  return (
    tagName === "INPUT" ||
    tagName === "TEXTAREA" ||
    (el as HTMLElement).isContentEditable === true
  );
};

export function useInputFocus<T extends HTMLElement = HTMLInputElement>(
  options: UseInputFocusOptions = {}
) {
  const { enableDelay = 60, restoreDelay = 120 } = options;
  const inputRef = useRef<T | null>(null);
  const focusTimer = useRef<number | null>(null);
  const blurTimer = useRef<number | null>(null);
  const focusState = useRef<FocusState>("normal");

  const clearTimer = (timerRef: MutableRefObject<number | null>) => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const debouncedEnableFocus = () => {
    if (!isTauriRuntime()) return;

    clearTimer(focusTimer);
    focusTimer.current = window.setTimeout(async () => {
      try {
        await invoke("activate_window_focus");
        focusState.current = "clipboard";
      } catch {
        // Ignore focus errors
      }
    }, enableDelay);
  };

  const debouncedRestoreFocus = () => {
    if (focusState.current === "normal") {
      return;
    }

    // macOS NSPanel: search blur often means focus moved to note/tag fields.
    // Restoring the previous app here steals the keyboard from those inputs.
    if (isMacPlatform()) {
      focusState.current = "normal";
      return;
    }

    clearTimer(blurTimer);
    blurTimer.current = window.setTimeout(async () => {
      if (isEditableElement(document.activeElement)) {
        return;
      }

      try {
        await restoreLastFocus();
        focusState.current = "normal";
      } catch {
        // Ignore restore errors
      }
    }, restoreDelay);
  };

  useEffect(() => {
    const element = inputRef.current;
    if (!element) return;

    const handleFocus = () => {
      debouncedEnableFocus();
    };

    const handleBlur = () => {
      debouncedRestoreFocus();
    };

    element.addEventListener("focus", handleFocus);
    element.addEventListener("blur", handleBlur);

    const checkInitialFocus = window.setTimeout(() => {
      if (document.activeElement === element) {
        debouncedEnableFocus();
      }
    }, 0);

    return () => {
      element.removeEventListener("focus", handleFocus);
      element.removeEventListener("blur", handleBlur);
      clearTimeout(checkInitialFocus);
      clearTimer(focusTimer);
      clearTimer(blurTimer);
    };
  }, [enableDelay, restoreDelay]);

  return inputRef;
}
