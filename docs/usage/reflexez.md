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

By default the change scope includes staged, unstaged, and untracked files. For
a branch or CI job, select an explicit base:

```bash
sensez reflexez . --base origin/main
```

## Inspect before executing

```bash
sensez reflexez . --plan
sensez reflexez . --plan --json
```

Every selected file includes a reason and dependency distance. Reasons distinguish
changed tests, direct dependencies, transitive dependencies, computed dynamic
imports, explicit full runs, and safety fallbacks.

## Runner integration

Reflexez detects local Vitest and Jest installations from `package.json` and
detects pytest tests by its standard file conventions. Force a runner only when
auto-detection is ambiguous:

```bash
sensez reflexez . --runner pytest
sensez reflexez . --runner vitest
```

Arguments after `--` are forwarded unchanged:

```bash
sensez reflexez . -- --maxfail=1      # pytest
sensez reflexez . -- --reporter=dot   # Vitest
```

Use explicit paths in editor and staged-file integrations:

```bash
sensez reflexez . --changed-file src/service.ts
```

## Safety model

Reflexez widens to the full discovered suite when a narrow plan cannot be proven
safe. Current global triggers include:

- dependency manifests and lockfiles;
- pytest configuration and `conftest.py`;
- Vitest/Jest configuration;
- Sensez configuration and common shared test setup/environment files;
- TypeScript module-resolution configuration;
- deleted source files or incomplete parsing;
- relevant dynamic imports whose targets cannot be resolved.

Literal `import()`, CommonJS `require()`, `importlib.import_module()`, and
`__import__()` targets use the normal import graph. Reflexez additionally resolves
simple constant indirection, concatenation, template/f-string patterns, and
`import.meta.glob` patterns. This work is isolated to the `reflexez` command;
ordinary `noze` scans do not invoke the dynamic-import pass.

The plan always reports the repository-wide count of opaque computed imports.
By default, only an opaque import in the selected impact path widens execution;
this matches the practical safety model of native changed-test tools. Use
`--strict-dynamic` in high-assurance CI to widen on any opaque computed import:

```bash
sensez reflexez . --base origin/main --strict-dynamic
```

`--full` is useful for validating the integration while retaining runner
discovery:

```bash
sensez reflexez . --full
```

Reflexez intentionally does not enable unsafe runner optimizations such as
disabling Vitest isolation or automatically adding pytest-xdist. Runner-level
parallelism can change fixture, process, and shared-resource semantics; pass
those options explicitly after measuring them in the repository.

