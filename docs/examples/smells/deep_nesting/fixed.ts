function approve(order: Order): boolean {
  if (!order.isPaid || order.isFlagged) {
    return false;
  }
  if (!order.customer.isActive) {
    return false;
  }
  if (!order.customer.isVerified) {
    return false;
  }
  return order.total < order.customer.limit;
}
