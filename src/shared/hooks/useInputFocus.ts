import { useEffect, useRef, type MutableRefObject } from "react";
import { restoreLastFocus } from "../lib/focus";
import { isMacPlatform } from "../lib/platform";

type FocusState = "normal" | "clipboard";

type UseInputFocusOptions = {
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
  const { restoreDelay = 120 } = options;
  const inputRef = useRef<T | null>(null);
  const blurTimer = useRef<number | null>(null);
  const focusState = useRef<FocusState>("normal");

  const clearTimer = (timerRef: MutableRefObject<number | null>) => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
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
      focusState.current = "clipboard";
    };

    const handleBlur = () => {
      debouncedRestoreFocus();
    };

    element.addEventListener("focus", handleFocus);
    element.addEventListener("blur", handleBlur);

    const checkInitialFocus = window.setTimeout(() => {
      if (document.activeElement === element) {
        focusState.current = "clipboard";
      }
    }, 0);

    return () => {
      element.removeEventListener("focus", handleFocus);
      element.removeEventListener("blur", handleBlur);
      clearTimeout(checkInitialFocus);
      clearTimer(blurTimer);
    };
  }, [restoreDelay]);

  return inputRef;
}
