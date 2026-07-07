from dataclasses import dataclass


@dataclass(frozen=True)
class SearchQuery:
    term: str
    page: int
    per_page: int
    sort: str


def search(query: SearchQuery) -> SearchResults:
    return index.find(query)
