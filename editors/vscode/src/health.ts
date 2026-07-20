import * as vscode from "vscode";

type PillarCount = { total: number; mustFix: number; warning: number };
type ChangeCount = { total: number; blocking: number };
type HealthFinding = {
  group?: string;
  label: string;
  detail: string;
  file: string;
  line: number;
  endLine?: number;
};
export type HealthSummary = {
  root: string;
  scope: string;
  currentChanges?: ChangeCount;
  cycles?: PillarCount;
  deadCode?: PillarCount;
  boundaries?: PillarCount;
  duplication?: PillarCount;
  smells?: PillarCount;
  cycleFindings?: HealthFinding[];
  deadCodeFindings?: HealthFinding[];
  boundaryFindings?: HealthFinding[];
  duplicationFindings?: HealthFinding[];
  smellFindings?: HealthFinding[];
};

type HealthNode = {
  label: string;
  description?: string;
  tooltip?: string;
  children?: HealthNode[];
  finding?: HealthFinding;
};

export class ChangeHealthProvider implements vscode.TreeDataProvider<HealthNode> {
  private readonly changed = new vscode.EventEmitter<HealthNode | undefined>();
  private summary: HealthSummary | undefined;
  readonly onDidChangeTreeData = this.changed.event;

  update(summary: HealthSummary): void {
    this.summary = summary;
    this.changed.fire(undefined);
  }

  clear(): void {
    this.summary = undefined;
    this.changed.fire(undefined);
  }

  getTreeItem(node: HealthNode): vscode.TreeItem {
    const state = node.children ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None;
    const item = new vscode.TreeItem(node.label, state);
    item.description = node.description;
    item.tooltip = node.tooltip ?? node.description;
    if (node.finding) {
      item.command = {
        command: "vscode.open",
        title: "Open sensez finding",
        arguments: [vscode.Uri.file(node.finding.file), { selection: location(node.finding) }]
      };
      item.tooltip = `${node.finding.detail}\n${node.finding.file}:${rangeLabel(node.finding)}`;
    }
    return item;
  }

  getChildren(node?: HealthNode): HealthNode[] {
    if (node) return node.children ?? [];
    return this.summary ? roots(this.summary) : [];
  }
}

export function statusText(summary: HealthSummary): { text: string; tooltip: string } {
  const { blocking } = changeCount(summary);
  return { text: blocking ? `z ${blocking} blocking` : "z", tooltip: tooltip(summary) };
}

function roots(summary: HealthSummary): HealthNode[] {
  const changes = changeCount(summary);
  const headline = changes.total
    ? `${changes.total} finding${plural(changes.total)}${changes.blocking ? ` · ${changes.blocking} blocking` : ""}`
    : "No maintainability issues in current changes";
  return [
    { label: "Current changes", description: headline },
    { label: "Existing repository findings", description: `${repositoryTotal(summary)} hidden` },
    {
      label: "Full repository report",
      children: [
        pillar("Import cycles", summary.cycles, summary.cycleFindings),
        pillar("Dead code", summary.deadCode, summary.deadCodeFindings),
        pillar("Boundary violations", summary.boundaries, summary.boundaryFindings),
        pillar("Structural duplicates", summary.duplication, summary.duplicationFindings),
        pillar("Design smells", summary.smells, summary.smellFindings)
      ]
    }
  ];
}

function pillar(label: string, count?: PillarCount, findings?: HealthFinding[]): HealthNode {
  const total = count?.total ?? 0;
  return {
    label,
    description: `${total} finding${plural(total)}`,
    children: groupedFindings(findings)
  };
}

function groupedFindings(findings?: HealthFinding[]): HealthNode[] | undefined {
  if (!findings) return undefined;
  const groups = new Map<string, HealthFinding[]>();
  const nodes: HealthNode[] = [];
  for (const finding of findings) {
    if (!finding.group) {
      nodes.push(findingNode(finding));
      continue;
    }
    const entries = groups.get(finding.group) ?? [];
    entries.push(finding);
    groups.set(finding.group, entries);
  }
  for (const [label, entries] of groups) {
    nodes.push({
      label,
      description: `${entries.length} location${plural(entries.length)}`,
      children: entries.map(findingNode)
    });
  }
  return nodes;
}

function findingNode(finding: HealthFinding): HealthNode {
  return { label: finding.label, description: relativeLocation(finding), finding };
}

function relativeLocation(finding: HealthFinding): string {
  const relative = vscode.workspace.asRelativePath(finding.file, false);
  return `${relative}:${rangeLabel(finding)}`;
}

function location(finding: HealthFinding): vscode.Range {
  const startLine = Math.max(0, finding.line - 1);
  const endLine = Math.max(startLine + 1, finding.endLine ?? finding.line);
  return new vscode.Range(startLine, 0, endLine, 0);
}

function rangeLabel(finding: HealthFinding): string {
  const start = finding.line || 1;
  return finding.endLine && finding.endLine > start ? `${start}-${finding.endLine}` : `${start}`;
}

function repositoryTotal(summary: HealthSummary): number {
  return [summary.cycles, summary.deadCode, summary.boundaries, summary.duplication, summary.smells]
    .filter((count): count is PillarCount => count !== undefined)
    .reduce((sum, count) => sum + count.total, 0);
}

function tooltip(summary: HealthSummary): string {
  const { blocking, total } = changeCount(summary);
  return `sensez Change Health\n\n${total} finding${plural(total)} in current changes\n${blocking} blocking\n\nClick to open Change Health.`;
}

function plural(value: number): string { return value === 1 ? "" : "s"; }

function changeCount(summary: HealthSummary): ChangeCount {
  return summary.currentChanges ?? { total: 0, blocking: 0 };
}
