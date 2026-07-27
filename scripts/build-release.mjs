import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const platform = process.argv[2]?.trim().toLowerCase();
const channel = process.argv[3]?.trim().toLowerCase();
const extraArgs = process.argv.slice(4).filter(arg => arg !== "--prepare-only");
const prepareOnly = process.argv.includes("--prepare-only");

if (!["windows", "macos"].includes(platform)) {
  console.error("Usage: node scripts/build-release.mjs <windows|macos> <stable|beta> [--prepare-only] [tauri build args]");
  process.exit(1);
}
if (!["stable", "beta"].includes(channel)) {
  console.error("Release channel must be stable or beta.");
  process.exit(1);
}

if (existsSync(resolve(".env")) && typeof process.loadEnvFile === "function") {
  process.loadEnvFile(resolve(".env"));
}

const env = {
  ...process.env,
  TIEZ_PLATFORM: platform,
  TIEZ_RELEASE_CHANNEL: channel
};
const node = process.execPath;
const generatedFiles = [
  resolve("package.json"),
  resolve("src-tauri/tauri.conf.json"),
  resolve("src-tauri/Cargo.toml"),
];
const originalContents = new Map(
  generatedFiles.map(file => [file, readFileSync(file)]),
);

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit", env });
  if (result.status !== 0) {
    const error = new Error(`${command} failed`);
    error.exitCode = result.status ?? 1;
    throw error;
  }
}

let exitCode = 0;
try {
  run(node, [resolve("scripts/configure-release.mjs")]);
  run(node, [resolve("scripts/validate-release.mjs")]);

  if (prepareOnly) {
    console.log(`[build-release] Prepared ${channel}/${platform} without building`);
  } else {
    const tauriCli = process.platform === "win32"
      ? resolve("node_modules/.bin/tauri.cmd")
      : resolve("node_modules/.bin/tauri");
    run(tauriCli, ["build", ...extraArgs]);
    console.log(`[build-release] Finished ${channel}/${platform}`);
  }
} catch (error) {
  console.error(`[build-release] ${error.message}`);
  exitCode = error.exitCode || 1;
} finally {
  for (const [file, content] of originalContents) {
    writeFileSync(file, content);
  }
  console.log("[build-release] Restored generated version files");
}

if (exitCode) process.exit(exitCode);
