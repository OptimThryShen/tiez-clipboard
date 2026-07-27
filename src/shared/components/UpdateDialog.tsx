import { Download, RefreshCw, X, ExternalLink } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import AppModal from "./AppModal";

interface UpdateDialogProps {
  isOpen: boolean;
  version: string;
  notes: string;
  important: boolean;
  releaseChannel: "stable" | "beta";
  downloadProgress: number | null;
  status: "idle" | "checking" | "downloading" | "ready" | "error";
  onUpdate: () => void;
  onClose: () => void;
}

const UpdateDialog = ({
  isOpen,
  version,
  notes,
  important,
  releaseChannel,
  downloadProgress,
  status,
  onUpdate,
  onClose
}: UpdateDialogProps) => {
  const handleOpenWebsite = () => {
    openUrl("https://tiez.name666.top/");
  };

  return (
    <AppModal
      open={isOpen}
      onClose={onClose}
      panelClassName="modal-panel modal-panel-update"
      closeOnOverlayClick={status !== "downloading"}
    >
      <div className="modal-update-header">
        <div className="modal-update-header-info">
          <h3 className="modal-update-title">发现新版本</h3>
          <span className="modal-update-version">v{version}</span>
          {releaseChannel === "beta" && <span className="modal-update-badge beta">Beta</span>}
          {important && <span className="modal-update-badge important">重要更新</span>}
        </div>
        <button
          type="button"
          onClick={onClose}
          disabled={status === "downloading"}
          className="modal-button icon-only"
          aria-label="关闭"
        >
          <X size={16} />
        </button>
      </div>

      <div className="modal-update-content">
        <div className="modal-update-notes custom-scrollbar">
          <p>
            {status === "error"
              ? "更新过程中遇到了错误。这可能是由于网络原因或系统权限导致，请尝试前往官网手动下载最新版本。"
              : (notes || "在这个版本中，我们带来了一些性能优化和体验改进。")}
          </p>
        </div>

        {(status === "downloading" || status === "ready") && (
          <div className="modal-update-progress">
            <div className="modal-update-progress-labels">
              <span>{status === "ready" ? "下载完成" : "正在下载更新..."}</span>
              <span>{downloadProgress === null ? "计算中" : `${Math.round(downloadProgress)}%`}</span>
            </div>
            <div className="modal-update-progress-track">
              <div
                className={`modal-update-progress-bar${downloadProgress === null ? " indeterminate" : ""}`}
                style={downloadProgress === null ? undefined : { width: `${downloadProgress}%` }}
              />
            </div>
          </div>
        )}

        <div className="modal-update-actions">
          <button
            type="button"
            onClick={onClose}
            disabled={status === "downloading"}
            className="modal-button"
          >
            {status === "error" ? "关闭" : "稍后"}
          </button>

          {status === "error" ? (
            <button type="button" onClick={handleOpenWebsite} className="modal-button primary">
              <ExternalLink size={16} />
              前往官网
            </button>
          ) : status === "ready" ? (
            <button type="button" onClick={onUpdate} className="modal-button success">
              <RefreshCw size={18} className="modal-spin-slow" />
              立即重启
            </button>
          ) : (
            <button
              type="button"
              onClick={onUpdate}
              disabled={status === "downloading"}
              className="modal-button primary"
            >
              {status === "downloading" ? (
                <>
                  <RefreshCw size={18} className="modal-spin" />
                  下载中...
                </>
              ) : (
                <>
                  <Download size={18} />
                  立即更新
                </>
              )}
            </button>
          )}
        </div>
      </div>
    </AppModal>
  );
};

export default UpdateDialog;
