def normalize(value: int) -> int:
    return value + 1


def middle(value: int) -> int:
    return normalize(value)


def outer(value: int) -> int:
    return middle(value)


result = outer(2)
