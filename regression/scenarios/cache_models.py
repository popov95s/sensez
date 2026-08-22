"""Typed data contracts shared by cache regression scenarios."""

from dataclasses import dataclass
from typing import TypedDict


class ScanReport(TypedDict):
    cycles: list[dict]
    dead_code: list[dict]
    duplication: list[dict]


@dataclass(frozen=True)
class ScenarioFiles:
    duplicate_left: str
    duplicate_right: str
    duplicate_left_body: str
    duplicate_right_body: str
    duplicate_body: str
    duplicate_changed_body: str
    duplicate_unique: str
    cycle_a: str
    cycle_b: str
    cycle_a_module: str
    cycle_b_module: str
    cycle_a_body: str
    cycle_b_initial_body: str
    cycle_b_changed_body: str
    provider: str
    provider_body: str
    provider_symbol: str
    consumer: str
    consumer_body: str
    consumer_other: str
