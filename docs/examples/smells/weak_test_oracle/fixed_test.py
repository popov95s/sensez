def save(value: int) -> int:
    return value


def test_saves() -> None:
    assert save(42) == 42
