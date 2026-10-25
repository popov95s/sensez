function approve(order: Order): boolean {
  if (order.isPaid) {
    if (!order.isFlagged) {
      if (order.customer.isActive) {
        if (order.customer.isVerified) {
          if (order.total < order.customer.limit) {
            return true;
          }
        }
      }
    }
  }
  return false;
}
