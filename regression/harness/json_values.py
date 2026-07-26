from __future__ import annotations

from dataclasses import dataclass
from typing import TypeAlias


@dataclass(frozen=True)
class JsonPath:
    segments: tuple[str, ...]


JsonPathLike: TypeAlias = JsonPath | tuple[str, ...]


def json_path(value: object, keys: JsonPathLike) -> object:
    current = value
    segments = keys.segments if isinstance(keys, JsonPath) else keys
    for key in segments:
        if not isinstance(current, dict) or key not in current:
            return None
        current = current[key]
    return current
