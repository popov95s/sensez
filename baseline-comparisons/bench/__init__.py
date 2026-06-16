"""Benchmark dashboard package for sensez vs. structural baselines.

Layers (each a single responsibility):
  model        — plain data structures (no logic)
  loading      — read results/ artifacts (runs.jsonl, sensez.json)
  solutions    — the solution registry + per-tool parse/compare strategies
  compare      — transform runs → comparison rows (pure)
  render       — present rows as terminal text / HTML

Add a solution by appending one `Solution` in `solutions.py` — no other
layer changes (open for extension, closed for modification).
"""
