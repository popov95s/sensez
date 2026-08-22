from dataclasses import dataclass


@dataclass(frozen=True)
class Measurement:
    mode: str
    selected_files: int | None
    executed_tests: int
    wall_seconds: float
    exit_code: int


@dataclass(frozen=True)
class Comparison:
    seconds_saved: float
    percent_saved: float
    speedup: float


def comparisons(
    measurements: list[Measurement], candidate: str = "reflexez"
) -> dict[str, Comparison]:
    by_mode = {item.mode: item for item in measurements}
    selected = by_mode[candidate]
    result = {}
    for mode, baseline in by_mode.items():
        if mode == candidate:
            continue
        saved = baseline.wall_seconds - selected.wall_seconds
        result[mode] = Comparison(
            seconds_saved=round(saved, 3),
            percent_saved=round(saved / baseline.wall_seconds * 100, 1),
            speedup=round(baseline.wall_seconds / selected.wall_seconds, 2),
        )
    return result

