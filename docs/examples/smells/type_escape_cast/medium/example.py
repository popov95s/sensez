def shipping_label(order: Order) -> str:
    return f"{order.parcel.carrier} {order.parcel.tracking_code}"  # type: ignore
