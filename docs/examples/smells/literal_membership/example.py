def is_allowed(status: str) -> bool:
    return status in ["draft", "paid", "void"]
