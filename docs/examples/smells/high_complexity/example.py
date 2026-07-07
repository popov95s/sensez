def choose_refund(order: Order) -> RefundDecision:
    if order.cancelled:
        return deny_refund(order, "cancelled")
    if order.fraud_review:
        return manual_review(order)
    if order.days_since_purchase > 30:
        return deny_refund(order, "expired")
    if order.damaged:
        return full_refund(order)
    if order.missing_items:
        return partial_refund(order)
    if order.vip_customer:
        return goodwill_credit(order)
    return deny_refund(order, "not_eligible")
