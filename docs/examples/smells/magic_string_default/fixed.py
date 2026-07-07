def display_name(name: str | None) -> str:
    if name is None:
        raise ValueError("display name is required")
    return name
