function loadPayload(raw: Input) {
  try {
    const names = raw.names || [];
    const options = raw.options || {};
    return { names, options };
  } catch {
    return { names: [], options: {} };
  }
}
