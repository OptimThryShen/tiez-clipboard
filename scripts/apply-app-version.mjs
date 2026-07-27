import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const versionsPath = resolve(root, "versions.json");
const versions = JSON.parse(readFileSync(versionsPath, "utf8"));

function detectPlatform() {
  const arg = process.argv[2]?.trim().toLowerCase();
  if (arg === "macos" || arg === "mac") return "macos";
  if (arg === "windows" || arg === "win") return "windows";

  const env = process.env.TIEZ_PLATFORM?.trim().toLowerCase();
  if (env === "macos" || env === "mac") return "macos";
  if (env === "windows" || env === "win") return "windows";

  if (process.platform === "darwin") return "macos";
  if (process.platform === "win32") return "windows";

  return null;
}

function detectChannel() {
  const arg = process.argv[3]?.trim().toLowerCase();
  const env = process.env.TIEZ_RELEASE_CHANNEL?.trim().toLowerCase();
  const channel = arg || env || "stable";
  return channel === "stable" || channel === "beta" ? channel : null;
}

const platform = detectPlatform();
if (!platform) {
  console.error("[apply-app-version] Unable to detect a supported platform. Set TIEZ_PLATFORM=macos or windows.");
  process.exit(1);
}
const channel = detectChannel();
if (!channel) {
  console.error("[apply-app-version] Release channel must be stable or beta.");
  process.exit(1);
}

// Backward compatibility for the previous flat versions.json format.
const channelVersions = versions[channel] || (channel === "stable" ? versions : null);
const version = process.env.TIEZ_APP_VERSION?.trim() || channelVersions?.[platform];

if (!version) {
  console.error(`[apply-app-version] No ${channel} version configured for platform: ${platform}`);
  process.exit(1);
}

const packageJsonPath = resolve(root, "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
packageJson.version = version;
writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConfPath = resolve(root, "src-tauri/tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf8"));
tauriConf.version = version;
writeFileSync(tauriConfPath, `${JSON.stringify(tauriConf, null, 2)}\n`);

const cargoTomlPath = resolve(root, "src-tauri/Cargo.toml");
const cargoToml = readFileSync(cargoTomlPath, "utf8");
const cargoVersionRe = /^version = ".*"$/m;
if (!cargoVersionRe.test(cargoToml)) {
  console.error("[apply-app-version] Failed to locate version in src-tauri/Cargo.toml");
  process.exit(1);
}
writeFileSync(
  cargoTomlPath,
  cargoToml.replace(cargoVersionRe, `version = "${version}"`),
);

console.log(`[apply-app-version] ${channel}/${platform} -> ${version}`);
