"""Plain data structures shared across the benchmark pipeline. No I/O, no logic."""

from dataclasses import dataclass, field


@dataclass(frozen=True, kw_only=True)
class TargetRecord:
    """Shared benchmark-target metadata carried by runs and dashboard rows."""

    target: str
    path: str
    files: int
    lines: int
    lang: str | None = None


@dataclass(frozen=True)
class Run(TargetRecord):
    """One recorded tool run, as stored per line in results/runs.jsonl."""

    tool: str
    pillar: str
    seconds: float
    out: str
    ts: int = 0


@dataclass
class SensezFindings:
    """Sensez' per-pillar counts plus the dead-code sets used for vulture parity."""

    cycles: int
    dead: int
    dup: int
    smells: int
    total: int
    tiers: dict = field(default_factory=dict)  # {High,Medium,Low} counts
    dead_set: set = field(default_factory=set)  # (basename, symbol)
    high_set: set = field(default_factory=set)  # (basename, symbol), High tier


@dataclass(frozen=True)
class Verdict:
    """A solution's comparison against sensez for one target (pillar-specific)."""

    sensez_n: int  # Sensez' count for the compared pillar
    comp_n: int | None  # solution's count, when the tool can report one
    parity: str  # human note on how the two relate


@dataclass(frozen=True)
class Comp:
    """A rendered comparison cell: a Verdict plus the tool's identity and time."""

    tool: str
    label: str
    secs: float
    sensez_n: int
    comp_n: int | None
    parity: str


@dataclass(frozen=True)
class Row(TargetRecord):
    """A target's sensez run plus every solution comparison for it."""

    sensez_secs: float
    comps: list  # list[Comp]
