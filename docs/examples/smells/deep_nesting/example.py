def approve(order: Order) -> bool:
    if order.is_paid:
        if not order.is_flagged:
            if order.customer.is_active:
                if order.customer.is_verified:
                    if order.total < order.customer.limit:
                        return True
    return False
