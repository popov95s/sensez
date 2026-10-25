import { messageOf } from "./example";

export function logFailure(failure: unknown): void {
  console.error(messageOf(failure));
}
