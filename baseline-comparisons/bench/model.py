"""Plain data structures shared across the benchmark pipeline. No I/O, no logic."""

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Run:
    """One recorded tool run, as stored per line in results/runs.jsonl."""

    target: str
    path: str
    files: int
    lines: int
    tool: str
    pillar: str
    seconds: float
    out: str
    lang: str = "?"
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

    sensez_n: object  # Sensez' count for the compared pillar
    comp_n: object  # solution's count (int, or a string sentinel)
    parity: str  # human note on how the two relate


@dataclass(frozen=True)
class Comp:
    """A rendered comparison cell: a Verdict plus the tool's identity and time."""

    tool: str
    label: str
    secs: float
    sensez_n: object
    comp_n: object
    parity: str


@dataclass
class Row:
    """A target's sensez run plus every solution comparison for it."""

    target: str
    lang: str
    files: int
    lines: int
    path: str
    sensez_secs: float
    comps: list  # list[Comp]
