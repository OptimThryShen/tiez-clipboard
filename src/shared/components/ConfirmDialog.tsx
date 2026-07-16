import AppModal from "./AppModal";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  theme: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onClose: () => void;
}

const ConfirmDialog = ({
  open,
  title,
  message,
  theme,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onClose
}: ConfirmDialogProps) => (
  <AppModal open={open} onClose={onClose} theme={theme} panelClassName="modal-panel confirm-dialog">
    <h3 className="confirm-dialog-title">{title}</h3>
    <p className="confirm-dialog-message">{message}</p>
    <div className="confirm-dialog-buttons">
      <button type="button" className="modal-button" onClick={onClose}>
        {cancelLabel}
      </button>
      <button type="button" className="modal-button primary" onClick={onConfirm}>
        {confirmLabel}
      </button>
    </div>
  </AppModal>
);

export default ConfirmDialog;
