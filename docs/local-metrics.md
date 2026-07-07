# Local metrics

Sensez records how it is used and how often its findings lead to real fixes, so it
can prove its worth and tune itself over time. **Everything is local-only.** It
lives under each repo's `.sensez/local-metrics/` (gitignored). There is no
OpenTelemetry, no network, no export of any kind. Disable per repo with:

```toml
[self_improvement]
enabled = false
```

Recording is automatic — the model is never asked to report (that would burn
tokens). The only user-initiated input is `triage_finding` (a debt /
false-positive verdict).

## Files on disk

| File | Contents | Growth bound |
| --- | --- | --- |
| `events.jsonl` | Append-only event log (one JSON object per line). The source of truth; everything else is derived. | Compacted on flush once it passes **4 MB**, keeping a **45-day** window. |
| `totals.json` | Aggregates rebuilt by folding events. Most fields are all-time; `reported_by_detector` is the latest scan's current finding set. | Bounded — keys are a finite `pillar/<detector>` space. |
| `last-scan.json` | Per-branch fingerprint baseline + resolved-history (for reintroduction detection). | Capped at **32 branches** (LRU); resolved-history entries expire after **30 days**. |
| `triage.json` | The user's debt / false-positive verdicts, keyed by finding fingerprint. | Grows only on explicit user verdicts. |

A finding's **fingerprint** is its structural identity (symbols, modules, rules,
participating files) — never line numbers or metric values — so moving code
around never looks like a fix. Its **detector** is `pillar/<kind>`
(e.g. `smells/god_module`, `dead_code/function`) or just the pillar for pillars
without sub-kinds (`cycles`, `boundaries`, `duplication`).

## Events (`events.jsonl`)

- **`scan`** — a scan ran. Carries `origin` (`tool` | `gate` | `cli`), per-detector
  `reported` counts for that scan, per-detector `resolved` (findings that vanished since the
  last full scan, with summed time-to-resolution), `reintroduced` (previously
  resolved findings that came back, with summed interval), `files`/`loc` (size),
  and a `config_hash`.
- **`search`** — a `search_docs` call: hit count, top score, bytes returned vs.
  referenced (context saved), and whether it was the first search on the repo.
- **`auto_resolve`** — the server re-ran the pipeline after files changed and
  found previously-reported findings gone (no agent involvement).
- **`gate_block`** — the end-of-turn gate blocked, with the fingerprints it
  flagged (input to block→fix conversion).
- **`outcome`** — a user triage verdict (`debt` / `false_positive`), keyed by
  detector so per-detector precision is derivable.

## `usage_report` output

Backed by `totals.json` plus read-time derivations (`brainz::report`):

- **`all_time.reported_by_detector`** — the latest persisted scan's current
  per-detector finding counts. Re-scanning unchanged code or switching away and
  back to the same branch must not inflate it as a lifetime counter.

- **`precision_by_detector`** — `(resolved + debt) / (resolved + debt + false_positive)`,
  with raw counts so a low-sample rate isn't mistaken for a confident one.
- **`mean_resolution_days`** — per detector + `_overall`; how fast findings get fixed.
- **`fix_reintroductions_by_detector`** — fixes that didn't stick:
  reintroduction count, reintroduction rate, and mean days a fix held.
- **`gate_funnel`** — scans by origin and the gate's share (edit-time enforcement).
- **`gate_conversion`** — of findings the gate blocked, how many were fixed vs.
  escaped open. `conversion_rate` is `null` until a full scan exists to measure
  against (the gate alone writes no baseline).
- **`search_health`** — zero-hit rate over all searches.
- **`self_health`** — `mean_scan_ms`, `ms_per_kfile`, `ms_per_kloc`: Sensez' own
  throughput, so a perf regression shows up in the same ledger as its findings.
- **`config_pressure`** — how often the effective config changed, plus current
  scope (exclusion globs, boundary rules, threshold).
- **`calibration`** — evidence-backed config-hygiene suggestions for the human
  to act on (Sensez never edits its own config).
- **`recent_30d`** — the trust signals recomputed over the trailing 30 days, so
  trend is visible, not just the all-time average.
- **`stale_findings`** / **`triaged`** — findings unresolved > 7 days (triage
  candidates) and the verdicts already recorded.

## How the metrics feed back

The collected data changes presentation, never the rules:

- **Ranking** — findings from detectors below 0.5 precision (with ≥ 4
  adjudications) sink to the back of scan and gate output, so trusted findings
  lead. The rule still fires; only order changes.
- **Gate escalation** — a blocked finding that was previously fixed (in the
  resolved-history) is flagged as a regression in the gate message.
- **Calibration** — noisy detectors, hotspots, and never-fired boundary rules
  are surfaced as suggestions; the human edits `sensez.toml`.
