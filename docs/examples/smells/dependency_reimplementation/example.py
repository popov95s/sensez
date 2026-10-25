def chunks(items, size):
    return [items[start : start + size] for start in range(0, len(items), size)]


print(chunks(["a", "b", "c"], 2))
