"""Parse docs-relevant metadata from the Rust source."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

try:
    from finding_types import SmellTerm
except ModuleNotFoundError:
    from docs.finding_types import SmellTerm

ROOT = Path(__file__).resolve().parents[1]
SMELL_KIND_RS = ROOT / "src/report/smell_kind.rs"
SMELL_KNOBS_RS = ROOT / "src/config/smells/knobs.rs"
SMELL_DEFAULTS_RS = ROOT / "src/config/smells/defaults.rs"
SMELL_RESOLVE_RS = ROOT / "src/config/smells/resolve.rs"
SMELL_RULES_RS = ROOT / "src/config/smells/rules.rs"
SMELL_STRICTNESS_RS = ROOT / "src/config/smells/strictness.rs"

LANGUAGE_ROWS = (
    (("python",), "Python"),
    (("javascript", "typescript"), "JS / TS"),
    (("rust",), "Rust"),
)


@dataclass(frozen=True)
class StrictnessLevel:
    title: str
    value: str
    description: str


@dataclass(frozen=True)
class StrictnessRule:
    smell: SmellTerm
    knob: str
    levels: tuple[StrictnessLevel, ...]


def camel_to_snake(value: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", value).lower()


def smell_kinds() -> set[SmellTerm]:
    source = SMELL_KIND_RS.read_text()
    body = source.split("pub fn as_str", 1)[1].split("impl std::fmt::Display", 1)[0]
    return {SmellTerm(value) for value in re.findall(r'=>\s*"([^"]+)"', body)}


def parse_rule_knobs() -> dict[SmellTerm, list[str]]:
    source = SMELL_RULES_RS.read_text()
    body = source.split("fn integer_rule_knobs", 1)[1].split("fn bool_rule_knobs", 1)[0]
    out = dict(integer_rule_knob_entries(body))
    return with_string_rule_knobs(out, strictness_rule_knob_entries())


def integer_rule_knob_entries(body: str) -> list[tuple[SmellTerm, list[str]]]:
    return [
        (SmellTerm(camel_to_snake(kind)), re.findall(r'"([^"]+)"', keys))
        for arm, keys in re.findall(r"(?s)(.*?)=>\s*&\[(.*?)\],", body)
        for kind in re.findall(r"SmellKind::(\w+)", arm)
    ]


def strictness_rule_knob_entries() -> list[tuple[SmellTerm, str]]:
    return [(smell, rule.knob) for smell, rule in strictness_rules().items()]


def with_string_rule_knobs(
    out: dict[SmellTerm, list[str]],
    entries: list[tuple[SmellTerm, str]],
) -> dict[SmellTerm, list[str]]:
    combined = {smell: knobs.copy() for smell, knobs in out.items()}
    for smell, knob in entries:
        combined.setdefault(smell, []).append(knob)
    return combined


def strictness_rules() -> dict[SmellTerm, StrictnessRule]:
    source = SMELL_STRICTNESS_RS.read_text()
    levels = strictness_level_sets(source)
    rules = {}
    rules_body = source.split("pub const STRICTNESS_RULES", 1)[1].split("];", 1)[0]
    pattern = (
        r"StrictnessRuleDoc\s*{\s*kind:\s*SmellKind::(\w+),\s*"
        r'knob:\s*"([^"]+)",\s*levels:\s*(\w+),'
    )
    for kind, knob, level_set in re.findall(pattern, rules_body):
        smell = SmellTerm(camel_to_snake(kind))
        rules[smell] = StrictnessRule(smell, knob, levels[level_set])
    return rules


def strictness_level_sets(source: str) -> dict[str, tuple[StrictnessLevel, ...]]:
    sets = {}
    pattern = r"pub const (\w+): &\[StrictnessLevelDoc\] = &\[(.*?)\];"
    for name, body in re.findall(pattern, source, re.DOTALL):
        sets[name] = tuple(
            StrictnessLevel(title, value, description)
            for title, value, description in re.findall(
                r"StrictnessLevelDoc\s*{\s*"
                r'title:\s*"([^"]+)",\s*'
                r'value:\s*"([^"]+)",\s*'
                r'description:\s*"([^"]+)",\s*'
                r"}",
                body,
                re.DOTALL,
            )
        )
    return sets


def default_knob_values() -> dict[str, str]:
    fields = _default_smell_fields()
    knobs = {}
    for kind, key, field in _rule_knob_fields():
        knobs[f"{kind}.{key}"] = fields.get(field, "10")
    return knobs


def default_enabled(smell_term: SmellTerm, language: str) -> bool:
    fields = _default_smell_fields()
    enabled = _default_for_kind(smell_term, fields)
    if language in {"javascript", "typescript"}:
        overrides = _js_ts_overrides()
        if smell_term in overrides["enabled"]:
            enabled = True
        if smell_term in overrides["disabled"]:
            enabled = False
    return enabled


def _default_for_kind(smell_term: SmellTerm, fields: dict[str, str]) -> bool:
    field = _enabled_fields().get(smell_term)
    if not field:
        return True
    return fields.get(field, "true") == "true"


def _default_smell_fields() -> dict[str, str]:
    source = SMELL_KNOBS_RS.read_text()
    default_impl = source.split("impl Default for Smells", 1)[1]
    body = default_impl.split("Smells {", 1)[1].split("}", 1)[0]
    return dict(re.findall(r"(\w+):\s*([^,\n]+),", body))


def _enabled_fields() -> dict[SmellTerm, str]:
    source = SMELL_RULES_RS.read_text()
    body = source.split("fn set_rule_enabled", 1)[1]
    return {
        SmellTerm(camel_to_snake(kind)): field
        for kind, field in re.findall(
            r"SmellKind::(\w+)\s*=>\s*smells\.(\w+)\s*=",
            body,
        )
    }


def _rule_knob_fields() -> list[tuple[SmellTerm, str, str]]:
    source = SMELL_RULES_RS.read_text()
    integer_body = source.split("fn apply_integer_knob", 1)[1].split("fn set", 1)[0]
    bool_body = source.split("fn apply_rule_knob", 1)[1].split("if bool_rule_knobs", 1)[0]
    integer_knobs = [
        (SmellTerm(camel_to_snake(kind)), key, field)
        for kind, key, field in re.findall(
            r'\(SmellKind::(\w+),\s*"([^"]+)"\)\s*=>\s*(?:\{\s*)?set\(&mut smells\.(\w+),',
            integer_body,
        )
    ]
    bool_knobs = [
        (SmellTerm(camel_to_snake(kind)), key, field)
        for kind, key, field in re.findall(
            r'\(SmellKind::(\w+),\s*"([^"]+)"\)\s*=>\s*\{\s*smells\.(\w+)\s*=',
            bool_body,
        )
    ]
    return integer_knobs + bool_knobs


def _js_ts_overrides() -> dict[str, set[SmellTerm]]:
    source = SMELL_DEFAULTS_RS.read_text()
    body = source.split("fn js_ts_default", 1)[1]
    enabled = {
        camel_to_snake(field)
        for field, value in re.findall(r"(\w+):\s*(true|false),", body)
        if value == "true"
    }
    disabled = {
        SmellTerm(camel_to_snake(kind))
        for kind in re.findall(r"SmellKind::(\w+)", body)
    }
    field_to_kind = {field: kind for kind, field in _enabled_fields().items()}
    return {
        "enabled": {
            field_to_kind[field] for field in enabled if field in field_to_kind
        },
        "disabled": disabled,
    }
