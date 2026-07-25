# Releasing Sensez for VS Code

The extension version is `editors/vscode/package.json`'s `version` field. The
repository intentionally uses one shared release version: Cargo, PyPI, npm,
and VS Code must all match the `vX.Y.Z` Git tag. A Marketplace version must be
greater than the previous published version and use `major.minor.patch`.

## One-time Marketplace setup

1. Create the `sensez` publisher in Visual Studio Marketplace. Its immutable
   identifier must match `package.json`'s `publisher` field.
2. Create a GitHub environment named `vscode-marketplace`; require review if
   you want a human approval before publishing.
3. Add a `VSCE_PAT` environment secret with Marketplace **Manage** scope.
   This is a transitional approach: migrate the workflow to Entra workload
   identity before global Azure DevOps PATs retire in December 2026.

## Release process

1. Run `just release X.Y.Z release`. It updates all package manifests,
   creates `vX.Y.Z`, and pushes the tag.
2. The tag starts both the existing release workflow and **Release VS Code
   extension**. The latter verifies its version matches the tag, builds every
   platform VSIX, and waits at the protected Marketplace environment before
   publishing.
3. If you need package-only artifacts before tagging, run **Release VS Code
   extension** manually with **publish** unchecked.

The workflow builds native packages for `darwin-arm64`, `darwin-x64`,
`linux-x64`, and `win32-x64`. Marketplace clients select the matching target
automatically.
