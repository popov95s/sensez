function search(term: string, page: number, perPage: number, sort: string): SearchResults {
  return index.find(term, page, perPage, sort);
}
