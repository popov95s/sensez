"""Typed models and serialization for evaluation run artifacts."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path

from models import (
    DiffStats,
    FindingCounts,
    QualityScore,
    ScanPayload,
    TaskSpec,
)


@dataclass(frozen=True)
class CommandResult:
    command: str
    returncode: int | None
    elapsed_seconds: float
    stdout: str
    stderr: str
    timed_out: bool


@dataclass(frozen=True)
class ScanResult:
    command: tuple[str, ...]
    returncode: int
    stderr: str
    payload: ScanPayload
    stdout: str | None = None


@dataclass(frozen=True)
class RunContext:
    task: TaskSpec
    variant: str
    run: int
    workspace: Path
    out_dir: Path
    oc_config_home: str
    oc_data_home: str


@dataclass(frozen=True)
class Environment:
    values: dict[str, str]


@dataclass(frozen=True)
class BenchmarkMetrics:
    task_id: str
    variant: str
    run: int
    agent_returncode: int | None
    agent_elapsed_seconds: float
    agent_timed_out: bool
    sensez_before: FindingCounts
    sensez_after: FindingCounts
    sensez_diff: FindingCounts
    sensez_delta_total: int
    quality: QualityScore
    sensez_tool_calls: int
    input_tokens: int
    output_tokens: int
    reasoning_tokens: int
    diff_stats: DiffStats
    test_returncode: int | None = None
    test_timed_out: bool | None = None


@dataclass(frozen=True)
class JsonDocument:
    value: object

    def write(self, path: Path) -> None:
        import json

        path.write_text(json.dumps(self.value, indent=2, sort_keys=True) + "\n")


def json_value(value: object) -> object:
    """Convert evaluation models into stable JSON-compatible structures."""
    if isinstance(value, ScanResult):
        result = {
            "command": list(value.command),
            "returncode": value.returncode,
            "stderr": value.stderr,
            "json": asdict(value.payload),
        }
        if value.stdout is not None:
            result["stdout"] = value.stdout
        return result
    if isinstance(value, BenchmarkMetrics):
        result = asdict(value)
        quality = result.pop("quality")
        result.update(quality)
        return result
    if hasattr(value, "__dataclass_fields__"):
        return asdict(value)
    return value
