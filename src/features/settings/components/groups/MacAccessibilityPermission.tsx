import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ComponentType, ReactNode } from "react";

interface LabelWithHintProps {
  label: string;
  hint?: string | ReactNode;
  hintKey: string;
}

interface MacAccessibilityPermissionProps {
  t: (key: string) => string;
  LabelWithHint: ComponentType<LabelWithHintProps>;
}

const MacAccessibilityPermission = ({ t, LabelWithHint }: MacAccessibilityPermissionProps) => {
  const [granted, setGranted] = useState<boolean | null>(null);
  const [checking, setChecking] = useState(false);

  const refresh = useCallback(async () => {
    setChecking(true);
    try {
      const ok = await invoke<boolean>("check_macos_permissions");
      setGranted(ok);
    } catch {
      setGranted(false);
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => {
      void refresh();
    }, 3000);

    let disposeFocus: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) void refresh();
      })
      .then((unlisten) => {
        disposeFocus = unlisten;
      })
      .catch(() => undefined);

    return () => {
      window.clearInterval(interval);
      disposeFocus?.();
    };
  }, [refresh]);

  const openSettings = () => {
    invoke("open_macos_accessibility_settings").catch(console.error);
  };

  return (
    <div className="setting-item">
      <LabelWithHint
        label={t("accessibility_permission")}
        hint={t("accessibility_permission_hint")}
        hintKey="accessibility_permission"
      />
      {checking ? (
        <span
          style={{
            fontSize: "12px",
            color: "var(--text-secondary)",
            whiteSpace: "nowrap"
          }}
        >
          {t("accessibility_permission_checking")}
        </span>
      ) : granted ? (
        <span
          style={{
            fontSize: "12px",
            fontWeight: 600,
            color: "#34c759",
            whiteSpace: "nowrap"
          }}
        >
          {t("accessibility_permission_granted")}
        </span>
      ) : (
        <button
          type="button"
          onClick={openSettings}
          style={{
            fontSize: "12px",
            fontWeight: 600,
            color: "#ff9f0a",
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
            whiteSpace: "nowrap"
          }}
        >
          {t("accessibility_go_authorize")}
        </button>
      )}
    </div>
  );
};

export default MacAccessibilityPermission;
