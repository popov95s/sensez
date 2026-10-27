def rename(label: str) -> str:
    return label.strip()


def test_keeps_label() -> None:
    assert rename(" box ") == "box"
