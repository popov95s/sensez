def add_item(items: list[Item], item: Item) -> list[Item]:
    next_items = list(items)
    next_items.append(item)
    return next_items
