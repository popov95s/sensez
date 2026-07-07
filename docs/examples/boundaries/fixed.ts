import { enqueue } from "./services/queue";

export function handleRequest(request: Request): Result {
  return enqueue(request.payload);
}
