function approve(order: Order): boolean {
  if (order.isPaid) {
    if (!order.isFlagged) {
      if (order.customer.isActive) {
        return order.total < order.customer.limit;
      }
    }
  }
  return false;
}
