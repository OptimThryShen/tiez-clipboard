import { useEffect, useId, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { acquireBlurGuard, releaseBlurGuard } from "../lib/focus";

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
  const modalId = useId();

  useEffect(() => {
    if (!open) return;
    const owner = `app-modal:${modalId}`;
    const acquired = acquireBlurGuard(owner).catch(() => {});
    return () => {
      void acquired.finally(() => releaseBlurGuard(owner).catch(() => {}));
    };
  }, [modalId, open]);

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
