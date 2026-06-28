from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import NamedTuple


JsonValue = None | bool | int | float | str | list["JsonValue"] | dict[str, "JsonValue"]


class CycleSortKey(NamedTuple):
    rank: int
    size: int
    stable: str


@dataclass(frozen=True)
class RootVariants:
    values: tuple[str, ...]


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


def load_json(path: Path) -> JsonValue:
    text = path.read_text()
    return json.loads(text)


def normalize_artifact(
    value: JsonValue, target_root: Path, target_name: str
) -> JsonValue:
    return _normalize(
        value, RootVariants(tuple(_root_variants(target_root))), target_name
    )


def _normalize(value: JsonValue, roots: RootVariants, target_name: str) -> JsonValue:
    if isinstance(value, dict):
        out: dict[str, JsonValue] = {}
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


def _root_variants(root: Path) -> tuple[str, ...]:
    values = {str(root), str(root.resolve())}
    for value in list(values):
        if value.startswith("/private/var/"):
            values.add(value.replace("/private/var/", "/var/", 1))
        elif value.startswith("/var/"):
            values.add(value.replace("/var/", "/private/var/", 1))
    return tuple(sorted(values, key=len, reverse=True))


def _normalize_string(text: str, roots: RootVariants, target_name: str) -> str:
    for root_text in roots.values:
        if root_text in text:
            text = text.replace(root_text, f"<{target_name}>")
    return text


def _sort_known(obj: dict[str, JsonValue]) -> dict[str, JsonValue]:
    out = dict(obj)
    for key in ("duplication", "dead_code", "boundaries", "smells"):
        if isinstance(out.get(key), list):
            out[key] = sorted(out[key], key=_stable_json)
    if isinstance(out.get("cycles"), list):
        out["cycles"] = sorted(out["cycles"], key=_cycle_key)
    if isinstance(out.get("tools"), list):
        out["tools"] = sorted(
            out["tools"],
            key=lambda item: item.get("name", "") if isinstance(item, dict) else "",
        )
    return {key: out[key] for key in sorted(out)}


def _cycle_key(value: JsonValue) -> CycleSortKey:
    if not isinstance(value, dict):
        return CycleSortKey(99, 0, _stable_json(value))
    action = value.get("action")
    modules = value.get("modules")
    size = len(modules) if isinstance(modules, list) else 0
    rank = {"must_fix": 0, "warning": 1, "advisory": 2, "info": 3}.get(action, 99)
    return CycleSortKey(rank, -size, _stable_json(value))


def _stable_json(value: JsonValue) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def dump_json(path: Path, value: JsonValue) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def normalize_file(
    source: Path,
    dest: Path,
    target_root: Path,
    target_name: str,
) -> None:
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
