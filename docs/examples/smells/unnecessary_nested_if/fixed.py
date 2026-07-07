def can_ship(order: Order) -> bool:
    if not order.paid:
        return False
    return order.address.is_valid
