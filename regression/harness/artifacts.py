import json
from pathlib import Path

from ..normalize import dump_json, normalize_artifact
from .models import Target


def dump_metrics_schema(path: Path, repo: Path) -> None:
    metric_dir = repo / ".sensez" / "local-metrics"
    files = sorted(item.name for item in metric_dir.glob("*") if item.is_file())
    events = []
    event_log = metric_dir / "events.jsonl"
    if event_log.exists():
        for line in event_log.read_text().splitlines():
            events.append(json.loads(line).get("event"))
    dump_json(path, {"files": files, "events": sorted(set(events))})


def dump_normalized(path: Path, value: object, repo: Path, target: Target) -> None:
    dump_json(path, normalize_artifact(value, repo, target["name"]))
