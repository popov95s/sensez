def delivery_status(order) -> DeliveryStatus:
    if order.is_paid:
        return DeliveryStatus.READY
    if order.payment_retry_allowed:
        return DeliveryStatus.RETRY
    return DeliveryStatus.BLOCKED
