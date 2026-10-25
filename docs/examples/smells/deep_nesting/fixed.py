def approve(order: Order) -> bool:
    if not order.is_paid or order.is_flagged:
        return False
    if not order.customer.is_active:
        return False
    if not order.customer.is_verified:
        return False
    return order.total < order.customer.limit
