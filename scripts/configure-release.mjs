import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const applyVersion = spawnSync(
  process.execPath,
  [resolve("scripts/apply-app-version.mjs")],
  { stdio: "inherit", env: process.env },
);
if (applyVersion.status !== 0) {
  process.exit(applyVersion.status ?? 1);
}

const isCi = process.env.GITHUB_ACTIONS === "true" || process.env.CI === "true";

const requiredInCi = [
  "TIEZ_UPDATE_ENDPOINT",
  "TIEZ_ANNOUNCEMENT_PING_URL",
  "VITE_AI_DEFAULT_API_KEY"
];

for (const key of requiredInCi) {
  if (isCi && !process.env[key]?.trim()) {
    console.error(`[configure-release] Missing required env: ${key}`);
    process.exit(1);
  }
}

const configPath = resolve("src-tauri/tauri.conf.json");
const config = JSON.parse(readFileSync(configPath, "utf8"));

const updateEndpoint =
  process.env.TIEZ_UPDATE_ENDPOINT?.trim() ||
  process.env.VITE_TIEZ_UPDATE_ENDPOINT?.trim();
const updaterPublicKey =
  process.env.TIEZ_UPDATER_PUBLIC_KEY?.trim() ||
  config.plugins?.updater?.pubkey?.trim();
const releaseChannel = (process.env.TIEZ_RELEASE_CHANNEL || "stable").trim().toLowerCase();
if (!["stable", "beta"].includes(releaseChannel)) {
  console.error("[configure-release] TIEZ_RELEASE_CHANNEL must be stable or beta");
  process.exit(1);
}

function buildDynamicUpdaterEndpoint(endpoint) {
  const withoutLegacyPlatform = endpoint
    .replace(/([?&])platform=\{\{target\}\}(&?)/, (_match, prefix, suffix) => {
      if (prefix === "?" && suffix) return "?";
      if (prefix === "&" && suffix) return "&";
      return "";
    })
    .replace(/[?&]$/, "");
  const params = [
    ["target", "{{target}}"],
    ["arch", "{{arch}}"],
    ["current_version", "{{current_version}}"],
    ["channel", releaseChannel],
  ];
  let result = withoutLegacyPlatform;

  for (const [key, value] of params) {
    const pattern = new RegExp(`(?:[?&])${key}=`);
    if (pattern.test(result)) {
      if (key === "channel") {
        result = result.replace(/([?&]channel=)[^&]*/i, `$1${value}`);
      }
      continue;
    }
    result += `${result.includes("?") ? "&" : "?"}${key}=${value}`;
  }
  return result;
}

const endpointSource = updateEndpoint || config.plugins?.updater?.endpoints?.[0];
if (endpointSource) {
  config.plugins ??= {};
  config.plugins.updater ??= {};
  const finalEndpoint = buildDynamicUpdaterEndpoint(endpointSource);
  config.plugins.updater.endpoints = [finalEndpoint];
}

if (updaterPublicKey) {
  config.plugins ??= {};
  config.plugins.updater ??= {};
  config.plugins.updater.pubkey = updaterPublicKey;
}

writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);

console.log("[configure-release] Updated Tauri release config");
console.log(`[configure-release] release channel: ${releaseChannel}`);
if (endpointSource) {
  console.log(`[configure-release] updater endpoint: ${config.plugins.updater.endpoints[0]}`);
}
if (updaterPublicKey) {
  console.log("[configure-release] updater public key: available");
}
