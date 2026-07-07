def summarize(items: Items) -> Summary:
    has_items = False
    total = 0
    for item in items:
        has_items = True
        total += item
    return Summary(has_items=has_items, total=total)
