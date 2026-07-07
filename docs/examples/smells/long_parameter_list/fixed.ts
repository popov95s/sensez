interface SearchQuery {
  term: string;
  page: number;
  perPage: number;
  sort: string;
}

function search(query: SearchQuery): SearchResults {
  return index.find(query);
}
