def normalize(value: int) -> int:
    return value + 1


def outer(value: int) -> int:
    return normalize(value)


result = outer(2)
