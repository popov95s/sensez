function displayName(name?: string): string {
  if (name === undefined) {
    throw new Error("display name is required");
  }
  return name;
}
