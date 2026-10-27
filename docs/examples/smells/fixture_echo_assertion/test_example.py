def rename(label: str) -> str:
    return label.strip()


def test_keeps_label() -> None:
    label = rename(" box ")
    assert label == label
