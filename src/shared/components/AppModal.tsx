import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";

export interface AppModalProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  theme?: string;
  panelClassName?: string;
  overlayClassName?: string;
  closeOnOverlayClick?: boolean;
}

const AppModal = ({
  open,
  onClose,
  children,
  theme,
  panelClassName = "modal-panel",
  overlayClassName = "",
  closeOnOverlayClick = true
}: AppModalProps) => {
  useEffect(() => {
    if (!open) return;
    invoke("set_ignore_blur", { ignore: true }).catch(() => {});
    return () => {
      invoke("set_ignore_blur", { ignore: false }).catch(() => {});
    };
  }, [open]);

  if (!open || typeof document === "undefined") return null;

  const themeClass = theme ? `theme-${theme}` : "";
  const panelClasses = [panelClassName, themeClass].filter(Boolean).join(" ");
  const overlayClasses = ["modal-overlay", overlayClassName].filter(Boolean).join(" ");

  return createPortal(
    <div
      className={overlayClasses}
      onClick={closeOnOverlayClick ? onClose : undefined}
    >
      <div
        className={panelClasses}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        {children}
      </div>
    </div>,
    document.body
  );
};

export default AppModal;
