function canShip(order: Order): boolean {
  if (!order.paid) {
    return false;
  }
  return order.address.isValid;
}
