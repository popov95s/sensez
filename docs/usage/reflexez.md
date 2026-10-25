# Affected tests with Reflexez

Reflexez runs the repository's existing pytest, Vitest, or Jest installation. It builds an impact plan and passes ordinary test-file paths to the runner.

Vitest already provides [`--changed`](https://main.vitest.dev/config/changed)
and Jest provides
[`--findRelatedTests`](https://jestjs.io/docs/cli#--findrelatedtests-spaceseparatedlistofsourcefiles);
pytest core provides path/name filters
and cached-failure selection, but not changed-source dependency selection.
Reflexez adds one runner-independent workflow across all three: Git change-scope
handling, transitive cross-file impact, computed dynamic-import analysis,
explainable JSON plans, and conservative full-suite fallbacks. No test config,
plugin, fixture, or runner command needs replacing.

```bash
sensez reflexez .
```

By default the change scope includes staged, unstaged, and untracked files; in
a repository without commits every file counts as changed. For
a branch or CI job, select an explicit base:

```bash
sensez reflexez . --base origin/main
```

`--base` adds the commits since the merge-base; staged, unstaged, and untracked
files are still included, so a local run also covers uncommitted edits.

## Inspect before executing

```bash
sensez reflexez . --plan
sensez reflexez . --plan --json
```

Every selected file includes a reason (`changed_test`, `direct_dependency`,
`transitive_dependency`, `dynamic_import`, `full_requested`, `safety_fallback`)
and a dependency distance: the number of import hops from a changed file. A
`dynamic_import` selection reached through a file whose computed import could
not be resolved counts that file as one hop, since the import may load any
module. The JSON plan also lists:

- `config_warnings`: runner-config keys that could not be read statically, with
  the file, key, reason, and the fallback used for that key;
- `opaque_import_files`: files whose computed imports could not be resolved;
- `unmapped_changes`: changed files that exist but are not in the import graph
  (for example an asset nothing imports), so "0 tests" is never silent.

## Runner integration

Reflexez detects Vitest and Jest from each `package.json`: a script that runs
`vitest` or `jest` selects that runner (both, when scripts run both); otherwise
a `vitest` or `jest` dependency does, Vitest first. Without either in any
`package.json`, a `vitest.config.*`, `vitest.workspace.*`, `vite.config.*` with
a `test` key, or `jest.config.*` at the root selects the runner. A
`package.json` that is not valid JSON is skipped with a config warning, and a
runner project with no discovered tests is not run.

pytest tests follow the repository's `pytest.ini`, `pyproject.toml` with
`[tool.pytest.ini_options]`, `tox.ini` or `setup.cfg`: its `testpaths` and
`python_files` decide what is collected. When the repository root has none, the
nearest such file above each test applies instead and pytest runs from its
directory (for example `backend/pyproject.toml`). Without any, the default
`test_*.py` / `*_test.py` patterns apply from the repository root. Force a runner only when auto-detection is
ambiguous:

```bash
sensez reflexez . --runner pytest
sensez reflexez . --runner vitest
```

Arguments after `--` are forwarded unchanged:

```bash
sensez reflexez . -- --maxfail=1      # pytest
sensez reflexez . -- --reporter=dot   # Vitest
```

Narrowed Vitest and Jest runs add `--passWithNoTests`, and a narrowed pytest run
treats exit code 5 (no tests collected) as success. Full runs keep the runner's
default, so an empty full suite still fails.

### Runner configuration

Reflexez reads Vitest and Jest configs statically (it never runs Node) to learn
which files are tests and which files are shared setup:

- Vitest: `vitest.config.*`, else `vite.config.*` (the `test` key), plus
  `vitest.workspace.*` and `test.projects`. Keys: `include`, `exclude`,
  `setupFiles`, `globalSetup`, `root`, `dir`. Project entries may be inline
  objects (with `extends`), config paths, directories, or globs with `!`
  negations.
- Jest: `jest.config.*`, else the `package.json` `"jest"` key, plus `projects`.
  Keys: `testMatch`, `testRegex`, `testPathIgnorePatterns`, `roots`, `rootDir`,
  `setupFiles`, `setupFilesAfterEnv`, `globalSetup`.

Supported config shapes: `export default` / `module.exports` objects,
`defineConfig`/`defineProject`/`defineWorkspace` with an object or a function
returning one, `mergeConfig(a, b)` (a relatively imported base config is
followed), module-level `const` indirection, string concatenation and
templates, spreads of local arrays and `configDefaults.exclude`, and
`path.resolve(__dirname, ...)`. Missing keys use the runner defaults.

A key that cannot be read (for example a helper call, an environment
conditional, or a non-relative import) is reported as a config warning, and
only that key falls back: `include`/`testMatch` to the `*.test.*`/`*.spec.*`/
`__tests__/` file-name heuristic, `exclude` to the runner default, setup keys to
no setup tracking. With `--strict-dynamic`, any such warning widens to a full
run.

Use explicit paths in editor and staged-file integrations:

```bash
sensez reflexez . --changed-file src/service.ts
```

A relative `--changed-file` is resolved against the current directory, or
against the repository root when no such file exists there (a deleted file).

## Safety model

Reflexez runs the full suite when a narrow plan cannot be proven safe. Current
global triggers include:

- dependency manifests and lockfiles;
- pytest configuration and `conftest.py`;
- Vitest/Jest configuration;
- Sensez configuration and conventionally named test setup/environment files
  (`setupTests.*`, `test-setup.*`, `.env.test*`);
- the configured `setupFiles`, `setupFilesAfterEnv`, and `globalSetup` modules,
  or anything they import;
- TypeScript module-resolution configuration;
- deleted or renamed source files (a rename counts as a deletion of the old
  path, including case-only renames); deleting other files, such as a README,
  does not trigger it;
- incomplete parsing of a file that may import something (a file whose text
  has no `import`, `require` or `from` only triggers it when it changed).

Literal `import()`, CommonJS `require()`, `importlib.import_module()`, and
`__import__()` targets use the normal import graph. Reflexez additionally resolves
simple constant indirection, concatenation, template/f-string patterns, and
`import.meta.glob` patterns, including arrays with `!` negations and `tsconfig`
path aliases. Static imports of assets (`.css`, `.json`, `.svg`, ..., including
`?raw`/`?url` suffixes and aliases) are edges too, so changing an asset selects
the tests that import it. Assets loaded through `new URL(...)` or filesystem reads
are not tracked. This work is isolated to the `reflexez` command; ordinary `noze`
scans do not invoke the dynamic-import pass.

The plan always lists files with opaque computed imports. An opaque import in a
file can load any module, so when source files change, reflexez also selects the
tests that can execute that file (or the file itself when it is a test). Opaque
imports in tooling that no test reaches do not widen the plan. Use
`--strict-dynamic` in high-assurance CI to run the full suite on any opaque
computed import, unreadable runner-config key, or changed source file outside
the import graph:

```bash
sensez reflexez . --base origin/main --strict-dynamic
```

Full runs, from `--full` or a safety fallback, invoke each runner once in its
root with no file arguments, so the runner's own discovery decides what runs.
`--full` is useful for validating the integration:

```bash
sensez reflexez . --full
```

Reflexez intentionally does not enable unsafe runner optimizations such as
disabling Vitest isolation or automatically adding pytest-xdist. Runner-level
parallelism can change fixture, process, and shared-resource semantics; pass
those options explicitly after measuring them in the repository.

