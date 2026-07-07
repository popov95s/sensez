def status_for(order: Order) -> str:
    rules = (
        (order.cancelled, "cancelled"),
        (order.refunded, "refunded"),
        (order.failed, "failed"),
        (order.paid, "paid"),
    )
    for applies, status in rules:
        if applies:
            return status
    return "open"
