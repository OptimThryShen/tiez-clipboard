import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const combinations = [
  ["windows", "stable"],
  ["windows", "beta"],
  ["macos", "stable"],
  ["macos", "beta"],
  // Restore the repository's default generated state.
  ["windows", "stable"],
];

for (const [platform, channel] of combinations) {
  const result = spawnSync(
    process.execPath,
    [resolve("scripts/build-release.mjs"), platform, channel, "--prepare-only"],
    {
      stdio: "inherit",
      env: {
        ...process.env,
        CI: "false",
        GITHUB_ACTIONS: "false",
      },
    },
  );
  if (result.status !== 0) process.exit(result.status ?? 1);
}

console.log("[validate-release-matrix] stable and beta metadata passed for Windows and macOS");
