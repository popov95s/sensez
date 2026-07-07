function groupedNames(groups: Groups, names: string[]): string[] {
  const result: string[] = [];
  for (const group of groups) {
    names.sort();
    result.push(group.name);
  }
  return result;
}
