function statusFor(order: Order): string {
  if (order.cancelled) {
    return "cancelled";
  }
  if (order.refunded) {
    return "refunded";
  }
  if (order.failed) {
    return "failed";
  }
  if (order.paid) {
    return "paid";
  }
  return "open";
}
