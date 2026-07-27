import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "./tauriRuntime";

export type ReleaseChannel = "stable" | "beta";

export interface ReleaseBuildInfo {
  channel: ReleaseChannel;
  portable: boolean;
}

const DEFAULT_BUILD_INFO: ReleaseBuildInfo = {
  channel: "stable",
  portable: false,
};

let buildInfoPromise: Promise<ReleaseBuildInfo> | null = null;

export const getReleaseBuildInfo = (): Promise<ReleaseBuildInfo> => {
  if (!isTauriRuntime()) return Promise.resolve(DEFAULT_BUILD_INFO);
  if (!buildInfoPromise) {
    const request: Promise<ReleaseBuildInfo> = invoke<ReleaseBuildInfo>("get_release_build_info")
      .then((value): ReleaseBuildInfo => ({
        channel: value.channel === "beta" ? "beta" : "stable",
        portable: Boolean(value.portable),
      }))
      .catch((error) => {
        console.warn("[Release] Failed to read build info:", error);
        return DEFAULT_BUILD_INFO;
      });
    buildInfoPromise = request;
    return request;
  }
  return buildInfoPromise;
};
