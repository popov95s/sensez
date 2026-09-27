def save(value: int) -> int:
    return value


def test_saves() -> None:
    saved = save(1)
    save(saved)
