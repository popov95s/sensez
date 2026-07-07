function approve(order: Order): boolean {
  if (!order.isPaid) {
    return false;
  }
  if (order.isFlagged) {
    return false;
  }
  if (!order.customer.isActive) {
    return false;
  }
  return order.total < order.customer.limit;
}
