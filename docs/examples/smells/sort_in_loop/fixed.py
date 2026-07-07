def grouped_names(groups: Groups) -> Names:
    names = []
    for group in groups:
        names.extend(group.names)
    names.sort()
    return names
