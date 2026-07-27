import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const versions = JSON.parse(readFileSync(resolve(root, "versions.json"), "utf8"));
const packageJson = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
const tauriConfig = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
const cargoToml = readFileSync(resolve(root, "src-tauri/Cargo.toml"), "utf8");
const cargoVersion = cargoToml.match(/^version = "([^"]+)"$/m)?.[1];
const semverPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

function detectPlatform() {
  const value = (process.env.TIEZ_PLATFORM || process.argv[2] || "").trim().toLowerCase();
  if (["windows", "win"].includes(value)) return "windows";
  if (["macos", "mac", "darwin"].includes(value)) return "macos";
  throw new Error("Set TIEZ_PLATFORM to windows or macos before validating a release");
}

function fail(message) {
  console.error(`[validate-release] ${message}`);
  process.exitCode = 1;
}

const platform = detectPlatform();
const channel = (process.env.TIEZ_RELEASE_CHANNEL || process.argv[3] || "stable").trim().toLowerCase();
if (!["stable", "beta"].includes(channel)) {
  fail(`unsupported release channel: ${channel}`);
}
const channelVersions = versions[channel] || (channel === "stable" ? versions : null);
const expectedVersion = channelVersions?.[platform];
if (!semverPattern.test(expectedVersion || "")) {
  fail(`versions.json contains an invalid ${channel}/${platform} SemVer: ${expectedVersion || "(empty)"}`);
}
if (channel === "stable" && expectedVersion?.includes("-")) {
  fail(`stable version cannot contain a prerelease suffix: ${expectedVersion}`);
}
if (channel === "beta" && !expectedVersion?.includes("-")) {
  fail(`beta version must contain a prerelease suffix: ${expectedVersion}`);
}

for (const [source, value] of [
  ["package.json", packageJson.version],
  ["tauri.conf.json", tauriConfig.version],
  ["Cargo.toml", cargoVersion],
]) {
  if (value !== expectedVersion) {
    fail(`${source} is ${value || "(empty)"}, expected ${expectedVersion} for ${platform}`);
  }
}

const endpoint = tauriConfig.plugins?.updater?.endpoints?.[0] || "";
for (const placeholder of ["{{target}}", "{{arch}}", "{{current_version}}"]) {
  if (!endpoint.includes(placeholder)) {
    fail(`updater endpoint is missing ${placeholder}`);
  }
}
if (!endpoint.startsWith("https://")) {
  fail("updater endpoint must use HTTPS");
}
if (!endpoint.includes(`channel=${channel}`)) {
  fail(`updater endpoint does not target the ${channel} channel`);
}

const tag = (process.env.GITHUB_REF_NAME || "").trim();
if (tag && process.env.GITHUB_REF_TYPE === "tag") {
  const expectedTag = channel === "beta"
    ? (platform === "windows" ? `beta-v${expectedVersion}` : `beta-mac-v${expectedVersion}`)
    : (platform === "windows" ? `v${expectedVersion}` : `mac-v${expectedVersion}`);
  if (tag !== expectedTag) {
    fail(`tag ${tag} does not match ${platform} version; expected ${expectedTag}`);
  }
}

if (!process.exitCode) {
  console.log(`[validate-release] ${channel}/${platform} ${expectedVersion} is ready`);
}
