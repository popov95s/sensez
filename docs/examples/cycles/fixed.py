def g() -> int:
    return h() + 1


def h() -> int:
    return _base() - 1


def _base() -> int:
    return 1
