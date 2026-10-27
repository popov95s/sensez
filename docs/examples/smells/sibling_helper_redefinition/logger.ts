function messageOf(failure: unknown): string {
  return failure instanceof Error ? failure.message : String(failure);
}

export function logFailure(failure: unknown): void {
  console.error(messageOf(failure));
}
