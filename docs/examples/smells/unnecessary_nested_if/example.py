def can_ship(order: Order) -> bool:
    if order.paid:
        if order.address.is_valid:
            return True
    return False
