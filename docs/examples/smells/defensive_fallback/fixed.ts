function loadPayload(raw: unknown): Payload {
  return PayloadSchema.parse(raw);
}
