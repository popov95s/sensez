# Reflexez performance benchmarks

Two harnesses cover both supported ecosystems. Both include Reflexez selection
overhead in its wall time and emit no source or test names.

## JavaScript and TypeScript

This harness compares the same source-change scope through:

1. Reflexez planning plus execution of its selected files.
2. Vitest's native `related` command.
3. The complete Vitest suite when `--full` is supplied.

It emits only aggregate counts, exit status, and timings. It never records the
repository path, repository identity, source, package names, test names, or file
names, so it is safe to use against private benchmark repositories.

```bash
python -m regression.performance.reflexez_benchmark \
  /path/to/repository \
  --sensez target/release/sensez \
  --diff HEAD~5..HEAD \
  --full
```

Run on an otherwise idle machine. Repeat at least three times and report the
median. The selection overhead is included in Reflexez's total wall time.

## Python

The Python harness compares Reflexez with full pytest and pytest-testmon. Testmon
requires an initial full instrumented run to create `.testmondata`; that setup
cost is reported separately. Before every measured Testmon run, the harness
restores the same clean baseline database so each sample observes the same edit.
The source file and generated Testmon database are restored on exit.

```bash
python -m regression.performance.pytest_benchmark \
  /path/to/disposable/public-checkout \
  --sensez target/release/sensez \
  --changed-file src/package/module.py \
  --before 'flag = True' \
  --after 'flag: bool = True' \
  --runs 3
```

Use a disposable, clean checkout without an existing `.testmondata`. Install
the project test dependencies and `pytest-testmon==2.2.0` first. Results for the
pinned public Python benchmark are in `latest.python.json`; unlike the private
JavaScript benchmark, its project and commit are intentionally disclosed.

