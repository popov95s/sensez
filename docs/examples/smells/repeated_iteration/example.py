def summarize(items: Items) -> Summary:
    has_items = any(items)
    total = sum(items)
    return Summary(has_items=has_items, total=total)
