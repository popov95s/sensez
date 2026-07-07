function city(order: Order): string {
  return order.customer.address.city.name;
}
