export function save(data: Payload): Result {
  return store.write(data);
}
