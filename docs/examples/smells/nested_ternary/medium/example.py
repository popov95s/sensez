def delivery_status(order: Order) -> DeliveryStatus:
    return (
        DeliveryStatus.READY
        if order.is_paid
        else DeliveryStatus.RETRY
        if order.payment_retry_allowed
        else DeliveryStatus.BLOCKED
    )
