function statusFor(order: Order): string {
  if (order.cancelled) {
    return "cancelled";
  } else if (order.refunded) {
    return "refunded";
  } else if (order.failed) {
    return "failed";
  } else if (order.paid) {
    return "paid";
  }
  return "open";
}
