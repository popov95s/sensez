def status_for(order: Order) -> str:
    if order.cancelled:
        return "cancelled"
    elif order.refunded:
        return "refunded"
    elif order.failed:
        return "failed"
    elif order.paid:
        return "paid"
    return "open"
