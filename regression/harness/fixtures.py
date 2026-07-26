from pathlib import Path

from .models import DeadCodeFixture


def apply_fixture(repo: Path, fixture: DeadCodeFixture, text: str) -> None:
    path = repo / fixture["path"]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    write_fixture_consumer(repo, fixture)


def extra_dead_code_fixture(fixture: DeadCodeFixture, symbol: str) -> DeadCodeFixture:
    path = Path(fixture["path"])
    extra_path = path.with_name(f"{path.stem}_extra{path.suffix}")
    text = (
        f"function {symbol}(): number {{\n  return 84;\n}}\n"
        if path.suffix == ".ts"
        else f"def {symbol}():\n    return 84\n"
    )
    return {
        "path": str(extra_path),
        "symbol": symbol,
        "detector": fixture["detector"],
        "text": text,
        "fix_text": "",
    }


def extra_symbol_for(fixture: DeadCodeFixture) -> str:
    if Path(fixture["path"]).suffix == ".ts":
        return "sensezNewGateHelper"
    return "sensez_new_gate_helper"


def write_fixture_consumer(repo: Path, fixture: DeadCodeFixture) -> None:
    path = Path(fixture["path"])
    live = live_symbol_for(path)
    if path.suffix == ".ts":
        consumer = path.with_name(f"{path.stem}-consumer{path.suffix}")
        module = f"./{path.with_suffix('').name}"
        text = f'import {{ {live} }} from "{module}";\nconsole.log({live});\n'
    else:
        consumer = path.with_name(f"{path.stem}_consumer{path.suffix}")
        module = path.with_suffix("").as_posix().replace("/", ".")
        text = f"from {module} import {live}\n\nprint({live}())\n"
    target = repo / consumer
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)


def live_symbol_for(path: Path) -> str:
    if path.suffix == ".ts":
        return "sensezRegressionLiveHelper"
    return "sensez_regression_live_helper"
