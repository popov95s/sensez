def approve(order: Order) -> bool:
    if not order.is_paid:
        return False
    if order.is_flagged:
        return False
    if not order.customer.is_active:
        return False
    return order.total < order.customer.limit
