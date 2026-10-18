def delivery_label(order: Order) -> str:
    return (
        format_label("gift" if order.is_gift else "standard")
        if order.is_paid
        else "pending"
    )
