export function enqueue(data: Payload): Result {
  return queue.push(data);
}
