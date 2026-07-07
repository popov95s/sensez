const MAX_RETRIES = 7;

function isRetryable(attempts: number): boolean {
  return attempts < MAX_RETRIES;
}
