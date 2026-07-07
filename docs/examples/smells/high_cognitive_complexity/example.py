def score(order: Order) -> int:
    total = 0
    if order.paid:
        if order.customer.active:
            if not order.customer.suspended:
                if order.total >= 500:
                    total += 25
                elif order.total >= 100:
                    total += 10
                else:
                    total += 1
    return total
