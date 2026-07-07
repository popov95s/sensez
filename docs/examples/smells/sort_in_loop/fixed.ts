function groupedNames(groups: Groups): string[] {
  const names: string[] = [];
  for (const group of groups) {
    names.push(...group.names);
  }
  names.sort();
  return names;
}
