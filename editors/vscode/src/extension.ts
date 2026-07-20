import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { resolveBinary } from "./binary";
import { ChangeHealthProvider, HealthSummary, statusText } from "./health";
import { enabled, healthEnabled, lspSettings, traceLevel } from "./settings";

let client: LanguageClient | undefined;
let healthProvider: ChangeHealthProvider | undefined;
let changeStatus: vscode.StatusBarItem | undefined;
let lastHealth: HealthSummary | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  healthProvider = new ChangeHealthProvider();
  changeStatus = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  changeStatus.command = "sensez.showChangeHealth";
  context.subscriptions.push(changeStatus, vscode.window.registerTreeDataProvider("sensez.changeHealth", healthProvider));
  await setHealthVisibility();
  context.subscriptions.push(vscode.commands.registerCommand("sensez.restart", () => start(context)));
  context.subscriptions.push(vscode.commands.registerCommand("sensez.showChangeHealth", showChangeHealth));
  context.subscriptions.push(vscode.commands.registerCommand("sensez.analyzeChanges", () => client?.sendRequest("workspace/executeCommand", { command: "sensez.rescan", arguments: [] })));
  context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(event => {
    if (event.affectsConfiguration("sensez")) {
      void setHealthVisibility();
      void start(context);
    }
  }));
  await start(context);
}

export async function deactivate(): Promise<void> { await client?.stop(); }

async function start(context: vscode.ExtensionContext): Promise<void> {
  await client?.stop();
  client = undefined;
  if (!enabled()) { setStatus("disabled"); return; }
  try {
    const command = await resolveBinary(context);
    const serverOptions: ServerOptions = { command, args: ["server", "stdio"], options: { env: { ...process.env, RUST_BACKTRACE: "0" } } };
    const clientOptions: LanguageClientOptions = {
      documentSelector: ["python", "javascript", "typescript", "typescriptreact", "rust"],
      synchronize: { configurationSection: "sensez" },
      initializationOptions: { sensez: lspSettings() },
      traceOutputChannel: vscode.window.createOutputChannel("sensez language server")
    };
    client = new LanguageClient("sensez", "sensez", serverOptions, clientOptions);
    client.setTrace(traceLevel());
    client.onNotification("sensez/status", (event: { state: string }) => setStatus(event.state));
    client.onNotification("sensez/health", (summary: HealthSummary) => updateHealth(summary));
    await client.start();
    setStatus("ready");
  } catch (error) {
    setStatus("unavailable");
    void vscode.window.showErrorMessage(`sensez could not start: ${String(error)}`);
  }
}

function setStatus(state: string): void {
  if (!changeStatus) return;
  if (state === "scanning") {
    changeStatus.text = "z";
    changeStatus.tooltip = "sensez is analysing current changes.";
  } else if (state === "error" || state === "unavailable") {
    changeStatus.text = "z";
    changeStatus.tooltip = "sensez could not complete analysis. Open the sensez language server output for details.";
  } else if (!lastHealth) {
    changeStatus.text = "z";
    changeStatus.tooltip = "sensez Change Health\n\nClick to open Change Health.";
  }
}

async function setHealthVisibility(): Promise<void> {
  if (!healthEnabled()) {
    healthProvider?.clear();
    changeStatus?.hide();
  } else if (vscode.workspace.workspaceFolders?.length) {
    changeStatus?.show();
  }
  await vscode.commands.executeCommand("setContext", "sensez.healthEnabled", healthEnabled());
}

async function showChangeHealth(): Promise<void> {
  await vscode.commands.executeCommand("workbench.view.extension.sensez");
}

function updateHealth(summary: HealthSummary): void {
  lastHealth = summary;
  healthProvider?.update(summary);
  if (!healthEnabled() || !changeStatus) return;
  const state = statusText(summary);
  changeStatus.text = state.text;
  changeStatus.tooltip = state.tooltip;
  changeStatus.show();
}
