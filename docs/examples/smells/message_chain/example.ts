function country(order: Order): string {
  return order.customer.address.city.country.name;
}
