from dataclasses import dataclass
from pathlib import Path

from ..harness.json_values import json_path
from ..mcp_client import McpClient, text_json


@dataclass(frozen=True)
class ReportFields:
    values: dict[str, object]

    def contains(self, key: str) -> bool:
        return key in self.values


def scan_full(client: McpClient, repo: Path) -> None:
    client.call_tool("noze_sniff", {"path": str(repo), "diff": False})


def brainz_report(client: McpClient, repo: Path) -> object:
    return text_json(client.call_tool("brainz_report", {"path": str(repo)}))


def optional_map(value: object, path: tuple[str, ...]) -> ReportFields:
    found = json_path(value, path)
    if found is None:
        return ReportFields({})
    if isinstance(found, dict):
        return ReportFields({str(key): item for key, item in found.items()})
    raise AssertionError(f"expected object at {'.'.join(path)}, got {found!r}")


def required_map(
    value: object,
    path: tuple[str, ...],
    target_name: str,
) -> ReportFields:
    found = optional_map(value, path)
    if not found.values:
        raise AssertionError(f"{target_name}: missing {'.'.join(path)}")
    return found


def required_int(
    value: object,
    path: tuple[str, ...],
    target_name: str,
) -> int:
    found = json_path(value, path)
    if not isinstance(found, int):
        raise AssertionError(f"{target_name}: expected integer at {'.'.join(path)}")
    return found


def detector_count(report: object, category: str, detector: str) -> int:
    value = json_path(report, ("all_time", category, detector, "count"))
    return value if isinstance(value, int) else 0
