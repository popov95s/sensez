@include control.md

You have access to a Sensez MCP server that detects: duplication, dead code,
import cycles, and design smells in your changes.

## Required workflow

1. Implement the task.

2. Call `noze_sniff` with `path` set to the workspace root. It returns
   JSON with `duplication`, `dead_code`, `cycles`, `smells` arrays.

3. The `duplication`, `dead_code`, `smells`, `cycles` arrays MUST be empty before you finish.
   If either is non-empty, fix using these rules, then call `noze_sniff`
   again. Repeat until both are empty.

   - Duplication: if you wrote the same logic in two places (including
     sync/async counterparts), extract it into a private helper. Example:
     instead of `if isinstance(x, Foo): v = x else: v = default()` in both
     `generate()` and `agenerate()`, write `def _resolve(x): return x if
     isinstance(x, Foo) else default()` and call it from both.
   - Dead code: if you introduced an unreferenced function/class, remove it.

4. Smells and cycles may be pre-existing. Check them, but only fix what you
   introduced.

5. The task is NOT complete until steps 2-4 are done.
