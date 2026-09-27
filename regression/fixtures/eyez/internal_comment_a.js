function internalFirst(options) {
  // Normalize request options with default headers and method before transport.
  const headers = { ...options.headers };
  const method = options.method ?? 'GET';
  if (method === 'POST') {
    headers['content-type'] = 'application/json';
  }
  return { ...options, headers, method };
}
