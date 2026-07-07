def city(order: Order) -> str:
    return order.customer.address.city.name
