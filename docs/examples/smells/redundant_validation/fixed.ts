function publish(message: Message | undefined) {
  if (message === undefined) return;
  prepare(message);
  send(message);
}
