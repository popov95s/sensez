function canShip(order: Order): boolean {
  if (order.paid) {
    if (order.address.isValid) {
      return true;
    }
  }
  return false;
}
