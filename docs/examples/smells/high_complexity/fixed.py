from collections.abc import Callable


RefundHandler = Callable[[Order], RefundDecision]


REFUND_POLICY: dict[str, RefundHandler] = {
    "cancelled": deny_cancelled,
    "fraud_review": manual_review,
    "expired": deny_expired,
    "damaged": full_refund,
    "missing_items": partial_refund,
    "vip_customer": goodwill_credit,
}


def choose_refund(order: Order) -> RefundDecision:
    handler = REFUND_POLICY.get(order.refund_reason(), deny_not_eligible)
    return handler(order)
