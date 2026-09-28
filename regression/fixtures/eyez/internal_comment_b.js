function internalSecond(input) {
  // Build normalized request options with headers and method defaults for transport.
  const headers = { ...input.headers };
  const method = input.method ?? 'GET';
  if (method === 'POST') {
    headers['content-type'] = 'application/json';
  }
  return { ...input, headers, method };
}
