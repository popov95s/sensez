function score(order: Order): number {
  let total = 0;
  if (order.paid) {
    if (order.customer.active) {
      if (!order.customer.suspended) {
        if (order.total >= 500) {
          total += 25;
        } else if (order.total >= 100) {
          total += 10;
        } else {
          total += 1;
        }
      }
    }
  }
  return total;
}
