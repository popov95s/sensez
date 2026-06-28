from __future__ import annotations

from typing import TypedDict

from .findings import Finding


class AnnotationProps(TypedDict):
    file: str
    line: str
    endLine: str
    title: str


def annotate(findings: list[Finding], level: str) -> None:
    for finding in findings:
        props: AnnotationProps = {
            "file": finding.file,
            "line": str(finding.start_line),
            "endLine": str(finding.end_line),
            "title": "Sensez duplication",
        }
        print(f"::{level} {_props(props)}::{_escape_message(finding.message)}")


def _props(values: AnnotationProps) -> str:
    return ",".join(f"{key}={_escape_property(value)}" for key, value in values.items())


def _escape_property(value: str) -> str:
    return (
        value.replace("%", "%25")
        .replace("\r", "%0D")
        .replace("\n", "%0A")
        .replace(":", "%3A")
        .replace(",", "%2C")
    )


def _escape_message(value: str) -> str:
    return value.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
