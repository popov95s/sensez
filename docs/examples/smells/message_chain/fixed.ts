class Order {
  shippingCountry(): string {
    return this.customer.shippingCountry();
  }
}

class Customer {
  shippingCountry(): string {
    return this.address.countryName();
  }
}

class Address {
  countryName(): string {
    return this.city.countryName();
  }
}

class City {
  countryName(): string {
    return this.country.name;
  }
}

function country(order: Order): string {
  return order.shippingCountry();
}
