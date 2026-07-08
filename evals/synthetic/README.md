# Synthetic Sensez Evals

This directory contains lightweight task manifests for A/B agent runs.

- `tasks.jsonl` covers broad Sensez pillars such as duplication, dead code, cycles, and smells.
- `tasks_smells.jsonl` is a direct detector smoke set. Several prompts intentionally name or strongly imply the code pattern to create.
- `tasks_sensez_exclusive.jsonl` is the subtler benchmark set. Its `summary` fields describe normal feature requests that tend to lure a baseline agent into maintainability smells, while `target_smell` and `trap_design` document the expected Sensez signal for analysis.

The shared runner reads the standard task fields and ignores the extra metadata, so the subtler set can be passed directly as `--tasks-file evals/synthetic/tasks_sensez_exclusive.jsonl`.
