def save_owner(owner: Owner) -> None:
    store.save(label=owner.label or "")
