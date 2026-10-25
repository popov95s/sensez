def order_total(raw: object) -> int:
    payload = Order.model_validate(raw)
    return sum(item.amount for item in payload.items)
