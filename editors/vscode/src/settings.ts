import * as vscode from "vscode";
import { Trace } from "vscode-languageclient/node";

export function enabled(): boolean {
  return vscode.workspace.getConfiguration("sensez").get<boolean>("enable", true);
}

export function healthEnabled(): boolean {
  return vscode.workspace.getConfiguration("sensez").get<boolean>("repositoryHealth.enabled", true);
}

export function lspSettings(): object {
  const config = vscode.workspace.getConfiguration("sensez");
  return {
    diagnostics: { level: config.get<string>("diagnostics.level", "must_fix") },
    analysis: { scope: config.get<string>("analysis.scope", "changed") },
    repositoryHealth: { enabled: healthEnabled() }
  };
}

export function traceLevel(): Trace {
  const value = vscode.workspace.getConfiguration("sensez").get<string>("trace.server", "off");
  if (value === "verbose") return Trace.Verbose;
  if (value === "messages") return Trace.Messages;
  return Trace.Off;
}
