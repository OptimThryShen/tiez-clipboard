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
  const [binaryPath, setBinaryPath] = useState("");

  const refresh = useCallback(async () => {
    setChecking(true);
    try {
      const [ok, path] = await Promise.all([
        invoke<boolean>("check_macos_permissions"),
        invoke<string>("get_macos_accessibility_binary_path")
      ]);
      setGranted(ok);
      setBinaryPath(path);
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

  const openSettings = async () => {
    await invoke("open_macos_accessibility_settings").catch(console.error);
    await invoke("request_macos_permissions").catch(console.error);
  };

  const copyBinaryPath = async () => {
    if (!binaryPath) return;
    try {
      await navigator.clipboard.writeText(binaryPath);
    } catch {
      console.error("Failed to copy accessibility binary path");
    }
  };

  return (
    <div className="setting-item" style={{ flexDirection: "column", alignItems: "stretch", gap: 8 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12 }}>
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
              color: "var(--status-success)",
              whiteSpace: "nowrap"
            }}
          >
            {t("accessibility_permission_granted")}
          </span>
        ) : (
          <button
            type="button"
            onClick={() => {
              void openSettings();
            }}
            style={{
              fontSize: "12px",
              fontWeight: 600,
              color: "var(--status-warning)",
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

      {!checking && !granted && (
        <div
          style={{
            fontSize: "12px",
            lineHeight: 1.5,
            color: "var(--text-secondary)",
            paddingLeft: 2
          }}
        >
          <div>{t("accessibility_permission_steps")}</div>
          {binaryPath && (
            <div style={{ marginTop: 6, wordBreak: "break-all" }}>
              <span>{t("accessibility_permission_binary")}</span>
              <code style={{ marginLeft: 4 }}>{binaryPath}</code>
              <button
                type="button"
                onClick={() => {
                  void copyBinaryPath();
                }}
                style={{
                  marginLeft: 8,
                  fontSize: "12px",
                  color: "var(--accent-color, #0a84ff)",
                  background: "none",
                  border: "none",
                  padding: 0,
                  cursor: "pointer"
                }}
              >
                {t("accessibility_copy_binary")}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default MacAccessibilityPermission;
