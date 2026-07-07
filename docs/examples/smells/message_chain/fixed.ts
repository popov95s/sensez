class Order {
  shippingCity(): string {
    return this.customer.shippingCity();
  }
}

class Customer {
  shippingCity(): string {
    return this.address.cityName();
  }
}

class Address {
  cityName(): string {
    return this.city.name;
  }
}

function city(order: Order): string {
  return order.shippingCity();
}
