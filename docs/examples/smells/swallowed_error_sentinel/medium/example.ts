function readSettings(text: string) {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}
