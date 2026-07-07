function isRetryable(attempts: number): boolean {
  return attempts < 7;
}
