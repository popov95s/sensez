import { access, cp, mkdir, rm } from "node:fs/promises";
import { arch, platform } from "node:process";
import { dirname, join, resolve } from "node:path";

const executable = platform === "win32" ? "sensez.exe" : "sensez";
const source = process.env.SENSEZ_BINARY ?? resolve("..", "..", "target", "release", executable);
const destination = join("bundled", `${platform}-${arch}`, executable);

try {
  await access(source);
} catch {
  throw new Error(`Sensez binary not found at ${source}. Build it with: cargo build --release --no-default-features --features all-langs,lsp`);
}

await rm(dirname(destination), { force: true, recursive: true });
await mkdir(dirname(destination), { recursive: true });
await cp(source, destination);
