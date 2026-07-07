def search(term: str, page: int, per_page: int, sort: str) -> SearchResults:
    return index.find(term, page, per_page, sort)
