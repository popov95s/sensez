function publish(message: Message | undefined) {
  if (message === undefined) return;
  prepare(message);
  if (message === undefined) return;
  send(message);
}
