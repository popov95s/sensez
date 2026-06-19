#!/usr/bin/env node

import { promises as fs } from "node:fs";
import path from "node:path";
import os from "node:os";
import { spawn } from "node:child_process";

const pkg = JSON.parse(
  await fs.readFile(new URL("../package.json", import.meta.url), "utf8"),
);

const owner = process.env.SENSEZ_GITHUB_OWNER ?? "popov95s";
const repo = process.env.SENSEZ_GITHUB_REPO ?? "sensez";
const version = process.env.SENSEZ_VERSION ?? pkg.version;
const baseUrl =
  process.env.SENSEZ_RELEASE_BASE_URL ??
  `https://github.com/${owner}/${repo}/releases/download/v${version}`;
const cacheDir =
  process.env.SENSEZ_CACHE_DIR ??
  path.join(os.homedir(), ".cache", "sensez", "js", version);

const asset = assetName(process.platform, process.arch);
if (!asset) {
  fail(`unsupported platform: ${process.platform} ${process.arch}`);
}

const binaryPath = path.join(cacheDir, asset);
await ensureBinary(binaryPath, `${baseUrl}/${asset}`);
await run(binaryPath, process.argv.slice(2));

function assetName(platform, arch) {
  if (platform === "linux") {
    if (arch === "x64") {
      return "sensez-js-linux-x64";
    }
    if (arch === "arm64") {
      return "sensez-js-linux-arm64";
    }
  }
  if (platform === "darwin") {
    if (arch === "x64") {
      return "sensez-js-darwin-x64";
    }
    if (arch === "arm64") {
      return "sensez-js-darwin-arm64";
    }
  }
  if (platform === "win32") {
    if (arch === "x64") {
      return "sensez-js-win32-x64.exe";
    }
  }
  return null;
}

async function ensureBinary(binaryPath, url) {
  try {
    await fs.access(binaryPath);
    return;
  } catch {
    // Fall through to download.
  }

  await fs.mkdir(path.dirname(binaryPath), { recursive: true });
  const response = await fetch(url);
  if (!response.ok) {
    fail(`download failed: ${response.status} ${response.statusText} from ${url}`);
  }

  const bytes = Buffer.from(await response.arrayBuffer());
  await fs.writeFile(binaryPath, bytes, { mode: 0o755 });
  if (process.platform !== "win32") {
    await fs.chmod(binaryPath, 0o755);
  }
}

function run(binaryPath, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(binaryPath, args, { stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (signal) {
        reject(new Error(`sense exited with signal ${signal}`));
        return;
      }
      resolve(code ?? 1);
    });
  }).then((code) => {
    process.exitCode = code;
  });
}

function fail(message) {
  console.error(`sensez npm launcher: ${message}`);
  process.exit(1);
}
