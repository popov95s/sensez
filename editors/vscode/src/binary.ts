import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as vscode from "vscode";

const executableName = process.platform === "win32" ? "sensez.exe" : "sensez";

export async function resolveBinary(context: vscode.ExtensionContext): Promise<string> {
  const configured = vscode.workspace.getConfiguration("sensez").get<string>("path", "").trim();
  if (configured && vscode.workspace.isTrusted) {
    await assertExecutable(configured);
    return configured;
  }
  const bundled = path.join(context.extensionPath, "bundled", `${process.platform}-${process.arch}`, executableName);
  await assertExecutable(bundled);
  return bundled;
}

async function assertExecutable(candidate: string): Promise<void> {
  try {
    await fs.access(candidate);
  } catch {
    throw new Error(`sensez executable was not found at ${candidate}. Install the matching VSIX or configure sensez.path in a trusted workspace.`);
  }
}
