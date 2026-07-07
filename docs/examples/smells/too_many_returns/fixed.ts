const ORDER_STATUS_RULES: Array<[keyof Order, string]> = [
  ["cancelled", "cancelled"],
  ["refunded", "refunded"],
  ["failed", "failed"],
  ["paid", "paid"],
];

function statusFor(order: Order): string {
  for (const [field, status] of ORDER_STATUS_RULES) {
    if (order[field]) {
      return status;
    }
  }
  return "open";
}
