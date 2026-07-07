def grouped_names(groups: Groups) -> Names:
    result = []
    for group in groups:
        names = list(group.names)
        names.sort()
        result.extend(names)
    return result
