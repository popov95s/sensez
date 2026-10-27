def country(order: Order) -> str:
    return order.customer.address.city.country.name
