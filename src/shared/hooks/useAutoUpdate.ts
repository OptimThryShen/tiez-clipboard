import { useState, useEffect, useCallback, useRef } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getReleaseBuildInfo, type ReleaseChannel } from "../lib/releaseBuild";
import { isTauriRuntime } from "../lib/tauriRuntime";

export type UpdateStatus = "idle" | "checking" | "downloading" | "ready" | "error";
export type UpdateCheckResultStatus = "available" | "up-to-date" | "error" | "portable";

export interface UpdateCheckResult {
  requestId?: string;
  status: UpdateCheckResultStatus;
  version?: string;
  message?: string;
}

interface ManualUpdateRequest {
  requestId?: string;
}

interface CheckOutcome extends UpdateCheckResult {
  update?: Update;
}

const CHECK_REQUEST_EVENT = "check-update-manually";
export const CHECK_RESULT_EVENT = "update-check-result";

export const useAutoUpdate = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [version, setVersion] = useState("");
  const [notes, setNotes] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(0);
  const [updateObj, setUpdateObj] = useState<Update | null>(null);
  const [important, setImportant] = useState(false);
  const [releaseChannel, setReleaseChannel] = useState<ReleaseChannel>("stable");
  const checkPromiseRef = useRef<Promise<CheckOutcome> | null>(null);
  const downloadedBytesRef = useRef(0);
  const totalBytesRef = useRef<number | null>(null);

  const performCheck = useCallback(async (): Promise<CheckOutcome> => {
    if (!isTauriRuntime()) return { status: "up-to-date" };

    const buildInfo = await getReleaseBuildInfo();
    setReleaseChannel(buildInfo.channel);
    if (buildInfo.portable) {
      setStatus("idle");
      return { status: "portable" };
    }

    try {
      setStatus("checking");
      const update = await check({
        headers: { "Cache-Control": "no-cache" },
        timeout: 10000,
      });

      if (!update) {
        setStatus("idle");
        return { status: "up-to-date" };
      }

      console.log(`[Update] New ${buildInfo.channel} version detected: ${update.version}`);
      setUpdateObj(update);
      setVersion(update.version);
      setNotes(update.body || "");
      setImportant(update.rawJson?.forceUpdate === true || update.rawJson?.important === true);
      setStatus("idle");
      setIsOpen(true);
      return { status: "available", version: update.version, update };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error("[Update] Failed to check for updates:", error);
      setStatus("error");
      return { status: "error", message };
    }
  }, []);

  const checkUpdate = useCallback(async (requestId?: string) => {
    if (!isTauriRuntime()) return;

    if (!checkPromiseRef.current) {
      checkPromiseRef.current = performCheck().finally(() => {
        checkPromiseRef.current = null;
      });
    }

    const outcome = await checkPromiseRef.current;
    if (requestId) {
      const { update: _update, ...result } = outcome;
      await emit<UpdateCheckResult>(CHECK_RESULT_EVENT, { ...result, requestId });
    }
  }, [performCheck]);

  const startUpdate = useCallback(async () => {
    if (!updateObj || status === "downloading") return;

    try {
      setStatus("downloading");
      setDownloadProgress(0);
      downloadedBytesRef.current = 0;
      totalBytesRef.current = null;

      await updateObj.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            totalBytesRef.current = event.data.contentLength ?? null;
            setDownloadProgress(totalBytesRef.current ? 0 : null);
            console.log(`[Update] Download started (${event.data.contentLength ?? "unknown"} bytes)`);
            break;
          case "Progress": {
            downloadedBytesRef.current += event.data.chunkLength;
            const total = totalBytesRef.current;
            if (total && total > 0) {
              setDownloadProgress(Math.min((downloadedBytesRef.current / total) * 100, 99));
            } else {
              setDownloadProgress(null);
            }
            break;
          }
          case "Finished":
            console.log("[Update] Download finished");
            setDownloadProgress(100);
            break;
        }
      });

      setStatus("ready");
      setDownloadProgress(100);
    } catch (error) {
      console.error("[Update] Failed to download or install update:", error);
      setStatus("error");
    }
  }, [status, updateObj]);

  const applyUpdate = useCallback(async () => {
    try {
      await relaunch();
    } catch (error) {
      console.error("[Update] Failed to relaunch:", error);
      setStatus("error");
      setIsOpen(true);
    }
  }, []);

  const closeUpdateDialog = useCallback(() => {
    if (status === "downloading") return;
    setIsOpen(false);
    setStatus("idle");
    setDownloadProgress(0);
    setImportant(false);
    if (updateObj) {
      updateObj.close().catch((error) => {
        console.warn("[Update] Failed to release updater resource:", error);
      });
      setUpdateObj(null);
    }
  }, [status, updateObj]);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    getReleaseBuildInfo().then((buildInfo) => {
      setReleaseChannel(buildInfo.channel);
    });

    const timer = setTimeout(() => {
      checkUpdate();
    }, 5000);

    let unlisten: (() => void) | undefined;
    listen<ManualUpdateRequest>(CHECK_REQUEST_EVENT, (event) => {
      checkUpdate(event.payload?.requestId);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      clearTimeout(timer);
      unlisten?.();
    };
  }, [checkUpdate]);

  return {
    isOpen,
    status,
    version,
    notes,
    important,
    releaseChannel,
    downloadProgress,
    onManualUpdate: checkUpdate,
    onStartDownload: startUpdate,
    onApplyUpdate: applyUpdate,
    onClose: closeUpdateDialog,
  };
};
