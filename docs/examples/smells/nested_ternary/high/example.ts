export function deliveryLabel(order: Order): string {
  return order.isPaid ? formatLabel(order.isGift ? "gift" : "standard") : "pending";
}
