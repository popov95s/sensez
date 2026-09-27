/** Normalize request options with default headers and method before transport. */
function prepareRequest(options) {
  const headers = { ...options.headers };
  const method = options.method ?? 'GET';
  if (method === 'POST') {
    headers['content-type'] = 'application/json';
  }
  return { ...options, headers, method };
}
