def status_for(order: Order) -> str:
    if order.cancelled:
        return "cancelled"
    if order.refunded:
        return "refunded"
    if order.failed:
        return "failed"
    if order.paid:
        return "paid"
    return "open"
