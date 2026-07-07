import { save } from "../db/client";

export function handleRequest(request: Request): Result {
  return save(request.payload);
}
