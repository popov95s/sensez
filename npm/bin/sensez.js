#!/usr/bin/env node

import { createRequire } from "node:module";
import { spawn } from "node:child_process";

const require = createRequire(import.meta.url);

const target = platformTarget(process.platform, process.arch);
if (!target) {
  fail(`unsupported platform: ${process.platform} ${process.arch}`);
}

const binaryPath = resolveBinary(target);
await run(binaryPath, process.argv.slice(2));

function platformTarget(platform, arch) {
  if (platform === "darwin") {
    if (arch === "arm64") {
      return { packageName: "sensez-darwin-arm64", binary: "bin/sensez" };
    }
    if (arch === "x64") {
      return { packageName: "sensez-darwin-x64", binary: "bin/sensez" };
    }
  }
  if (platform === "linux") {
    if (arch === "arm64") {
      return { packageName: "sensez-linux-arm64-gnu", binary: "bin/sensez" };
    }
    if (arch === "x64") {
      return { packageName: "sensez-linux-x64-gnu", binary: "bin/sensez" };
    }
  }
  if (platform === "win32" && arch === "x64") {
    return { packageName: "sensez-win32-x64-msvc", binary: "bin/sensez.exe" };
  }
  return null;
}

function resolveBinary(target) {
  try {
    return require.resolve(`${target.packageName}/${target.binary}`);
  } catch {
    fail(
      `missing native package ${target.packageName}; reinstall sensez with optional dependencies enabled`,
    );
  }
}

function run(binaryPath, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(binaryPath, args, { stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (signal) {
        reject(new Error(`sensez exited with signal ${signal}`));
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
