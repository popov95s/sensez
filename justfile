# Sensez release helpers.
#
# Install:   brew install just   (macOS)   |   apt: https://github.com/casey/just
# Run:       just                list every recipe
#            just check-versions verify all manifests agree with Cargo.toml
#            just bump 0.2.0     update every version field, refresh Cargo.lock
#            just release 0.2.0 pypi   bump + commit + tag + push, then dispatch

set shell := ["bash", "-uc"]
set dotenv-load := false

# Default: list available recipes.
default:
    @just --list

# --- Docs ------------------------------------------------------------------

# Regenerate the reference pages derived from the Rust source of truth.
docs-generate:
    uv run --no-project docs/generate.py

# Format and enforce multiline trailing commas in Python docs examples.
docs-examples-ruff:
    uv run --no-project --with ruff ruff format --no-cache docs/examples
    uv run --no-project --with ruff ruff check --no-cache --select COM812 docs/examples

# Format and enforce multiline trailing commas in TypeScript docs examples.
docs-examples-eslint:
    npm --prefix docs/examples install
    npm --prefix docs/examples run format
    npm --prefix docs/examples run check

# Build the MkDocs site locally.
docs-build:
    uv run --no-project docs/generate.py
    uv run --no-project --with ruff ruff format --no-cache --check docs/examples
    uv run --no-project --with ruff ruff check --no-cache --select COM812 docs/examples
    npm --prefix docs/examples install
    npm --prefix docs/examples run check
    NO_MKDOCS_2_WARNING=1 uv run --no-project --with mkdocs-material --with mike mkdocs build --strict

# Build a local mike version without pushing it.
docs-version version:
    uv run --no-project docs/generate.py
    NO_MKDOCS_2_WARNING=1 uv run --no-project --with mkdocs-material --with mike mike deploy --update-aliases "{{version}}" latest

# Serve the MkDocs site locally.
docs-serve:
    uv run --no-project docs/generate.py
    NO_MKDOCS_2_WARNING=1 uv run --no-project --with mkdocs-material --with mike mkdocs serve

# Serve the versioned docs tree locally after `just docs-version <version>`.
docs-version-serve:
    uv run --no-project --with mkdocs-material --with mike mike serve

# --- Benchmarks -------------------------------------------------------------

# Run timed benchmarks against pinned repos and compare to baseline.
bench:
    ./benchmarks/run.sh

# Run benchmarks and overwrite the baseline file (commit afterwards).
bench-update:
    SENSEZ_WRITE_BASELINE=1 ./benchmarks/run.sh

# --- Versioning ------------------------------------------------------------

# Verify every version field agrees with Cargo.toml. Mirrors the guard job in
# .github/workflows/release.yml so the same check runs locally before you push.
check-versions:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo_version=$(grep -E '^version = "' Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    py_version=$(grep -E '^version = "' pyproject.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    echo "Cargo.toml     = $cargo_version"
    echo "pyproject.toml = $py_version"
    if [ "$cargo_version" != "$py_version" ]; then
        echo "error: Cargo.toml and pyproject.toml disagree." >&2
        exit 1
    fi
    bad=0
    for f in npm/package.json npm/platform/*/package.json editors/vscode/package.json; do
        v=$(node -e "console.log(JSON.parse(require('fs').readFileSync(process.argv[1],'utf8')).version)" "$f")
        if [ "$v" != "$cargo_version" ]; then
            echo "  MISMATCH $f: $v"
            bad=1
        else
            echo "  ok        $f: $v"
        fi
    done
    if [ $bad -ne 0 ]; then
        echo "error: one or more npm packages are out of sync with Cargo.toml." >&2
        exit 1
    fi
    node -e '
        const fs = require("fs");
        const want = process.argv[1];
        const pkg = JSON.parse(fs.readFileSync("npm/package.json", "utf8"));
        const opts = pkg.optionalDependencies || {};
        for (const [name, v] of Object.entries(opts)) {
            if (v !== want) { console.error("  MISMATCH npm optionalDep " + name + "=" + v); process.exit(1); }
            console.log("  ok        npm optionalDep " + name + "=" + v);
        }
    ' "$cargo_version"
    echo "All version fields agree on $cargo_version."

# Update every version field to the new value, then refresh Cargo.lock.
# Leaves the working tree dirty so you can review the diff before committing.
# Usage: just bump 0.2.0
bump version: 
    @just _validate-semver "{{version}}"
    @just _write-version "{{version}}"
    cargo check --quiet
    @just check-versions
    @echo ""
    @echo "Bumped to {{version}}. Cargo.lock is refreshed."
    @echo "Review with:  git diff --stat"
    @echo "Commit with:  just release {{version}} <target>"

# Bump, commit, tag, and push. The tag is what you pass to the existing
# workflow_dispatch run on GitHub. The push is split so you can abort with
# Ctrl+C between the two git push commands if you change your mind.
# Usage: just release 0.2.0 pypi
release version target: _require-clean
    #!/usr/bin/env bash
    set -euo pipefail
    just _validate-semver "{{version}}"
    just _validate-target "{{target}}"
    just _write-version "{{version}}"
    cargo check --quiet
    git add Cargo.toml Cargo.lock pyproject.toml npm/package.json npm/platform/ editors/vscode/package.json editors/vscode/package-lock.json
    if ! git diff --cached --quiet; then
        git commit -m "bump version to {{version}}"
    else
        echo "Nothing to commit (already at {{version}}?)."
    fi
    git tag -a "v{{version}}" -m "v{{version}}"
    echo "Pushing branch and tag v{{version}}..."
    git push origin HEAD
    git push origin "v{{version}}"
    echo ""
    echo "Tag v{{version}} is now on origin. Go to GitHub Actions and trigger"
    echo "the Release workflow with target={{target}}."

# --- Internal helpers ------------------------------------------------------

# Refuse to bump a tree that has uncommitted changes to files that are NOT
# the version manifests. Version files are the ones we're about to edit, so
# changes there are expected; anything else would get swept into the bump
# commit and pollute the release.
_require-clean:
    #!/usr/bin/env bash
    set -euo pipefail
    untracked_excl=(--exclude-untracked -- \
        ':!Cargo.toml' ':!Cargo.lock' ':!pyproject.toml' ':!npm' ':!editors/vscode/package.json' ':!editors/vscode/package-lock.json' ':!justfile')
    if ! git diff --quiet "${untracked_excl[@]}" \
       || ! git diff --cached --quiet; then
        echo "error: working tree has uncommitted changes outside the version manifests." >&2
        echo "Commit or stash them first so the bump stays isolated." >&2
        git status --short
        exit 1
    fi

# Validate that $1 looks like a semver (with optional pre-release / build).
_validate-semver version:
    #!/usr/bin/env bash
    set -euo pipefail
    v="{{version}}"
    if [[ "$v" == v* ]]; then
        echo "error: pass the version without a leading 'v' (got '$v')." >&2
        exit 1
    fi
    if ! [[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
        echo "error: '$v' is not a valid semver (expected X.Y.Z, with optional -pre and +build)." >&2
        exit 1
    fi

# Validate that the release target is one of the workflow's accepted values.
_validate-target target:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
        testpypi|pypi|npm|release) ;;
        *) echo "error: target must be one of testpypi|pypi|npm|release (got '{{target}}')." >&2; exit 1 ;;
    esac

# Do the actual file edits. Cargo.toml is the source of truth; every other
# manifest is rewritten to match.
_write-version version:
    #!/usr/bin/env bash
    set -euo pipefail
    v="{{version}}"
    sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"$v\"/" Cargo.toml && rm Cargo.toml.bak
    sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"$v\"/" pyproject.toml && rm pyproject.toml.bak
    node -e '
        const fs = require("fs");
        const v = process.argv[1];
        const targets = [
            "npm/package.json",
            ...fs.readdirSync("npm/platform").map((n) => `npm/platform/${n}/package.json`),
            "editors/vscode/package.json",
        ];
        for (const f of targets) {
            const pkg = JSON.parse(fs.readFileSync(f, "utf8"));
            pkg.version = v;
            if (f === "npm/package.json" && pkg.optionalDependencies) {
                for (const name of Object.keys(pkg.optionalDependencies)) {
                    pkg.optionalDependencies[name] = v;
                }
            }
            fs.writeFileSync(f, JSON.stringify(pkg, null, 2) + "\n");
        }
        const lockPath = "editors/vscode/package-lock.json";
        const lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
        lock.version = v;
        if (lock.packages?.[""]) lock.packages[""].version = v;
        fs.writeFileSync(lockPath, JSON.stringify(lock, null, 2) + "\n");
    ' "$v"
