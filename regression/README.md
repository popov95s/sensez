# Sensez Regression Suite

This suite is a CI/local regression harness for Sensez itself. It is deliberately
kept outside `src/`: fixtures, scripts, snapshots, and generated results live
under top-level `regression/` so product code and external tool benchmarks stay
separate.

`baseline-comparisons/` answers "how does Sensez compare to other tools?"
This suite answers "did this change alter Sensez behavior, MCP behavior, or
local metrics relative to an accepted baseline?"

## Baseline Targets

Run the same scenario engine against multiple pinned repositories. Each target
declares its language profile and the scenarios it supports; the runner should
not special-case Flask or TypeScript.

Required PR targets:

- **Flask** (`py`): small real Python framework with decorators, package
  entrypoints, imports, tests, and framework patterns.
- **Zod** (`ts`): compact real TypeScript library with package exports,
  type-heavy modules, tests, and enough dependency structure to exercise the
  TypeScript/TSX profile.

Why these two:

- Both are small enough for PR CI and local developer runs.
- Together they protect the Python and TypeScript profiles, including the
  language path most sensitive to package/module resolution drift.
- Both are stable, recognizable repos where normalized finding changes are
  reviewable by humans.

Pin by commit, not branch. Store target specs in:

```text
regression/targets.toml
```

Suggested starting shape:

```toml
cache_root = "/tmp/sensez-regression-targets"

[[targets]]
name = "flask"
profile = "py"
url = "https://github.com/pallets/flask.git"
commit = "<accepted flask commit sha>"
scenarios = ["full", "mcp", "diff", "gate", "metrics", "triage"]

[[targets]]
name = "zod"
profile = "ts"
url = "https://github.com/colinhacks/zod.git"
commit = "<accepted zod commit sha>"
scenarios = ["full", "mcp", "diff", "gate", "metrics"]
```

Optional nightly targets can extend coverage without slowing PRs:

- `django`: larger Python stress target.
- `express`: JavaScript/CommonJS target if we want separate JS coverage.
- this repo itself: Rust target, useful for dogfooding but more volatile.

## Harness Engine

The runner should be target-driven:

1. Load `targets.toml`.
2. Select targets by `--target`, `--profile`, or `--all`.
3. Clone or reuse each pinned target under `cache_root`.
4. Check out the exact commit.
5. Run the target's optional `setup` command once per clean checkout.
6. Create a clean throwaway worktree for each scenario.
7. Remove target-local `.sensez/` before each scenario unless the scenario is
   explicitly measuring metric accumulation.
8. Run the scenario using profile-specific fixture edits.
9. Normalize volatile output.
10. Compare against `regression/baselines/<target>/`.
11. Write current artifacts under `regression/results/<target>/`.

The engine owns common behavior: checkout, worktree prep, MCP stdio, JSON
normalization, baseline comparison, timing thresholds, and `--accept`. Profiles
only provide language-specific fixture edits.

Profile fixture examples:

```toml
[profiles.py.dead_code_fixture]
path = "sensez_regression_fixture.py"
text = """
def sensez_regression_unused_helper():
    return 41 + 1
"""
fix_text = "print('fixed')\n"

[profiles.ts.dead_code_fixture]
path = "sensez-regression-fixture.ts"
text = """
function sensezRegressionUnusedHelper(): number {
  return 41 + 1;
}
"""
fix_text = "const sensezRegressionFixture = 42;\nconsole.log(sensezRegressionFixture);\n"
```

This keeps Flask and Zod equivalent from the runner's point of view.

## Baseline Artifacts

Commit only normalized baselines. Never commit cloned targets or local metrics.

```text
regression/
  README.md
  targets.toml
  run.sh
  mcp_client.py
  normalize.py
  analyze.py
  baselines/
    flask/
      full.noze.json
      full.noze.threshold40.json
      full.noze.max5.json
      diff.noze.json
      gate.block.json
      gate.allow.json
      mcp.tools.json
      brainz.after-full.json
      brainz.after-gate-fix.json
      metrics-files.schema.json
    zod/
      full.noze.json
      full.noze.threshold40.json
      full.noze.max5.json
      diff.noze.json
      gate.block.json
      gate.allow.json
      mcp.tools.json
      brainz.after-full.json
      brainz.after-gate-fix.json
      metrics-files.schema.json
  results/                          # gitignored current run output
```

The suite should fail when a normalized artifact changes unexpectedly. Updating
baselines is an explicit review step, not an automatic CI action.

## CI Entry Point

Local:

```bash
./regression/run.sh --target flask
./regression/run.sh --target zod
./regression/run.sh --all
```

CI:

```bash
cargo build --release --features mcp,all-langs
regression/run.sh --target flask --target zod --ci
```

The runner should print a compact diff summary and retain full current artifacts
under `regression/results/<target>/`.

## Full Scan Scenarios

Run the release binary, not `cargo run`, so CI exercises the packaged command
path.

Required scans per target:

```bash
target/release/sensez noze "$TARGET" --json
target/release/sensez noze "$TARGET" --max 5 --json
target/release/sensez noze "$TARGET" --all --json
target/release/sensez noze "$TARGET" --all --threshold 40 --json
target/release/sensez noze "$TARGET" --all --max 5 --json
```

Validate:

- JSON schema is stable and parseable.
- `meta.mode == "full"`.
- `meta.*_total` counts are present and internally consistent with arrays.
- `boundaries_configured` matches the effective config.
- Counts by pillar do not diverge from baseline unless intentionally accepted:
  `duplication`, `dead_code`, `cycles`, `boundaries`, `smells`.
- Finding identities remain stable after normalization.
- `--all` preserves the historical full candidate universe for detector
  regression baselines.
- default scans exercise the concise user/agent report.
- `--max 5` truncates returned arrays.
- Threshold-specific duplication counts match `full.noze.threshold40.json`.

Normalization should remove or canonicalize:

- Absolute paths to cloned targets.
- Wall-clock durations.
- Unordered object keys.
- Finding order when the order is not part of the contract.
- Branch, session, timestamp, and config hash fields exposed only through
  metrics.

Finding identity should keep:

- Target name and profile.
- Pillar and detector/kind.
- Label.
- File path relative to target root.
- Structural participants such as symbols, import endpoints, clone spans, and
  cycle members.

## MCP Server Scenarios

Start one MCP stdio server per scenario so metrics flush behavior is observable
and isolated.

```bash
target/release/sensez mcp serve
```

The MCP client should send newline-delimited JSON-RPC and assert both response
shape and side effects.

Required calls per target:

1. `initialize`
   - Assert protocol version is returned.
   - Assert `serverInfo.name == "sensez"`.

2. `tools/list`
   - Assert the tool catalog includes:
     `noze_sniff`, `get_configuration_summary`, `noze_gate`,
     `noze_explain`, `brainz_triage`, `brainz_report`.
   - If the binary is built with `eyez`, assert `eyez_search_docs`; otherwise
     assert it is absent.

3. `tools/call noze_sniff`
   - Arguments: `{ "path": "$TARGET", "limit": 20 }`.
   - Assert `isError == false`.
   - Parse `content[0].text` as JSON.
   - Compare normalized report to that target's `full.noze.json`.
   - Assert `.sensez/local-metrics/` now exists.

4. `tools/call noze_sniff` with `diff=true`
   - Run after the profile-specific fixture edit.
   - Assert `meta.mode == "diff"`.
   - Assert only findings touching changed lines are returned.
   - Assert full-graph totals remain available in metadata.

5. `tools/call brainz_report`
   - Assert the response is JSON text with `session` and `all_time`.
   - Compare selected metric fields after normalization.

The MCP suite should also send EOF and wait for the process to exit. This
exercises final metric flush and `recapture()` on shutdown.

## Diff Gate Scenarios

Create deterministic changes in a throwaway worktree. Commit the pristine
baseline first so `git diff HEAD` is meaningful.

### Gate Allows Clean Tree

Call:

```json
{
  "name": "noze_gate",
  "arguments": {
    "path": "$TARGET",
    "stop_hook_active": false
  }
}
```

Expected:

- Response text is `{}`.
- No gate block event is recorded.

### Gate Blocks Introduced Finding

Apply the target profile's dead-code fixture, then call `noze_gate`.

Expected:

- Response text parses as JSON.
- `decision == "block"`.
- `reason` contains `sensez diff-scan`.
- Top findings include the introduced dead-code finding.
- `.sensez/local-metrics/events.jsonl` contains a `scan` event with
  `origin == "gate"`.
- A `gate_block` event is recorded with at least one fingerprint.
- `last-scan.json` exists.

### Gate Allows Repeated Stop

Call `noze_gate` again with:

```json
{ "stop_hook_active": true }
```

Expected:

- Response text is `{}`.
- No duplicate block is recorded for the same stop-hook pass.

### Gate Tracks Fix Conversion

Apply the target profile's fix text so the finding disappears, then shut down
the MCP server or call a full `noze_sniff` through MCP.

Expected in `brainz_report`:

- `gate_conversion.blocked_findings > 0`.
- `gate_conversion.resolved > 0` once a full baseline exists.
- The target's dead-code detector count increases under
  `resolved_by_detector`.
- `mean_resolution_days` includes that detector after normalization.

## Local Metrics Scenarios

Metrics are part of the regression contract because MCP users rely on them for
trust signals. Validate files and derived report fields, but normalize volatile
timestamps and session ids.

Files expected under target `.sensez/local-metrics/`:

- `events.jsonl`
- `totals.json`
- `last-scan.json`
- `triage.json` only after explicit triage

Event log assertions:

- Every line parses as one JSON object.
- `scan` events include `ts`, `session`, `branch`, `ms`, `origin`, `reported`,
  `resolved`, `reintroduced`, `files`, `loc`, and optional `config_hash`.
- `gate_block` events include `fingerprints`.
- `outcome` events appear only after `brainz_triage`.
- `search` events are only required in an `eyez` feature job.

Totals assertions:

- `scans` increases after `noze_sniff` and `noze_gate`.
- `scans_by_origin.tool` and `scans_by_origin.gate` reflect the scenario.
- `reported_by_detector` contains stable detector keys for the target.
- `scan_ms_total`, `scan_files_total`, and `scan_loc_total` are positive.
- `gate_blocks` increases only on blocking gate calls.
- `config_changes` remains zero unless the scenario intentionally changes
  config.

Derived `brainz_report` assertions:

- `precision_by_detector` appears only after fixes or triage produce evidence.
- `mean_resolution_days` is present after a finding is resolved.
- `recidivism_by_detector` is empty in the default run.
- `gate_funnel.by_origin` includes tool and gate counts.
- `gate_conversion.conversion_rate` is `null` before a full baseline exists and
  numeric afterward.
- `search_health.zero_hit_rate` is stable in the non-`eyez` run.
- `self_health.mean_scan_ms`, `ms_per_kfile`, and `ms_per_kloc` are positive
  but compared with thresholds rather than exact values.
- `config_pressure.scope` matches the effective config.
- `stale_findings` is empty for freshly introduced-and-fixed scenarios.

## Triage Scenario

Run triage on targets whose profile fixture produces a deterministic label. It
is required for Flask and optional for Zod until the TypeScript dead-code label
is confirmed stable.

Call:

```json
{
  "name": "brainz_triage",
  "arguments": {
    "path": "$TARGET",
    "pillar": "dead_code",
    "match": "$PROFILE_FIXTURE_SYMBOL",
    "verdict": "debt",
    "note": "regression fixture"
  }
}
```

Expected:

- Response starts with `marked debt:`.
- `triage.json` is created.
- `events.jsonl` contains an `outcome` event.
- `totals.outcomes["debt:<detector>"]` increases.
- `brainz_report.precision_by_detector.<detector>.debt` increases.

Then call the same tool with `verdict = "clear"` and assert the triage file no
longer marks the finding as debt. Keep the clear step in local verification; CI
can stop after the debt assertion if the fixture is thrown away.

## Divergence Policy

Classify changes before updating baselines:

- **Bug/regression**: unexpected count changes, missing findings, new MCP
  errors, invalid JSON, missing metrics, or gate decisions changing from block
  to allow.
- **Intentional detector behavior change**: finding identity/count changes with
  a linked PR explanation and reviewed updated baseline.
- **Performance regression**: full scan exceeds baseline by more than 25% on
  PR CI, or 40% on shared hosted runners. Compare medians from three runs.
- **Metrics regression**: required event fields disappear, counters fail to
  increment, or derived report fields become inconsistent.
- **Allowed volatility**: elapsed milliseconds, absolute temp paths, timestamps,
  sessions, branch names in detached CI, and raw config hashes.

Baseline update flow:

```bash
regression/run.sh --target flask --accept
regression/run.sh --target zod --accept
git diff regression/baselines
```

Reviewers should inspect normalized JSON diffs and the runner's summary before
accepting. The PR description should state whether the change is detector
behavior, output schema, MCP behavior, metrics behavior, or performance.

## CI Matrix

PR-required:

- macOS or Linux latest stable Rust.
- `cargo build --release --features mcp,all-langs`.
- Flask and Zod regression suites.

Nightly:

- Flask, Zod, Django, optional Express.
- Three timing runs per target.
- Optional `eyez` feature job for `eyez_search_docs` and `search` metrics.
- Existing `baseline-comparisons` tool-comparison benchmark report.

Release-blocking:

- PR-required suite.
- Nightly matrix on all supported language profiles.
- `uv run maturin build --release` if packaging changed.

## Implementation Notes

- Keep the runner language boring: shell for orchestration, Python for JSON
  normalization/comparison and MCP stdio.
- Do not depend on network in normal CI beyond initial cache population. Pinned
  target checkouts should be cacheable by commit.
- Keep generated results gitignored.
- Print concise failures in CI, but retain full current artifacts under
  `results/` for local inspection.
- Use relative paths inside snapshots so developers can run the suite from any
  machine.
- Make `--accept` impossible in CI unless an explicit environment variable is
  set, for example `SENSEZ_ACCEPT_BASELINE=1`.
