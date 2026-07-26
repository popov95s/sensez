from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "regression" / "targets.toml"
RESULTS = ROOT / "regression" / "results"
BASELINES = ROOT / "regression" / "baselines"
