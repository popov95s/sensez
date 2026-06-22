from __future__ import annotations

import json
from pathlib import Path
from typing import Any


VOLATILE_KEYS = {
    "ts",
    "session",
    "branch",
    "ms",
    "config_hash",
    "last_config_hash",
    "since_unix",
    "secs_total",
    "first_used",
    "last_used",
    "mean_scan_ms",
    "ms_per_kfile",
    "ms_per_kloc",
    "scan_ms_total",
}


def load_json(path: Path) -> Any:
    text = path.read_text()
    return json.loads(text)


def normalize_artifact(value: Any, target_root: Path, target_name: str) -> Any:
    return _normalize(value, _root_variants(target_root), target_name)


def _normalize(value: Any, roots: list[str], target_name: str) -> Any:
    if isinstance(value, dict):
        out: dict[str, Any] = {}
        for key, item in value.items():
            if key in VOLATILE_KEYS:
                continue
            out[key] = _normalize(item, roots, target_name)
        return _sort_known(out)
    if isinstance(value, list):
        return [_normalize(item, roots, target_name) for item in value]
    if isinstance(value, str):
        return _normalize_string(value, roots, target_name)
    return value


def _root_variants(root: Path) -> list[str]:
    values = {str(root), str(root.resolve())}
    for value in list(values):
        if value.startswith("/private/var/"):
            values.add(value.replace("/private/var/", "/var/", 1))
        elif value.startswith("/var/"):
            values.add(value.replace("/var/", "/private/var/", 1))
    return sorted(values, key=len, reverse=True)


def _normalize_string(text: str, roots: list[str], target_name: str) -> str:
    for root_text in roots:
        if root_text in text:
            text = text.replace(root_text, f"<{target_name}>")
    return text


def _sort_known(obj: dict[str, Any]) -> dict[str, Any]:
    for key in ("duplication", "dead_code", "cycles", "boundaries", "smells"):
        if isinstance(obj.get(key), list):
            obj[key] = sorted(obj[key], key=_stable_json)
    if isinstance(obj.get("tools"), list):
        obj["tools"] = sorted(obj["tools"], key=lambda item: item.get("name", ""))
    return {key: obj[key] for key in sorted(obj)}


def _stable_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def dump_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def normalize_file(source: Path, dest: Path, target_root: Path, target_name: str) -> None:
    dump_json(dest, normalize_artifact(load_json(source), target_root, target_name))


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("dest", type=Path)
    parser.add_argument("--target-root", type=Path, required=True)
    parser.add_argument("--target-name", required=True)
    args = parser.parse_args()
    normalize_file(args.source, args.dest, args.target_root, args.target_name)
