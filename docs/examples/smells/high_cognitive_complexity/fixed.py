NO_SCORE = 0
ENTERPRISE_ORDER_THRESHOLD = 500
LARGE_ORDER_THRESHOLD = 100
ENTERPRISE_ORDER_SCORE = 25
LARGE_ORDER_SCORE = 10
STANDARD_ORDER_SCORE = 1


def score(order: Order) -> int:
    if not can_score(order):
        return NO_SCORE
    return score_total(order.total)


def can_score(order: Order) -> bool:
    if not order.paid:
        return False
    if not order.customer.active:
        return False
    if order.customer.suspended:
        return False
    return True


def score_total(total: int) -> int:
    if total >= ENTERPRISE_ORDER_THRESHOLD:
        return ENTERPRISE_ORDER_SCORE
    if total >= LARGE_ORDER_THRESHOLD:
        return LARGE_ORDER_SCORE
    return STANDARD_ORDER_SCORE
