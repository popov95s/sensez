def summarize_order(order):
    subtotal = totals.get(order.id)
    taxes = tax_table.get(order.region)
    fees = fee_table.get(order.region)
    if subtotal > 100:
        subtotal = subtotal - 5
    if order.region == "EU":
        taxes = taxes + 2
    return subtotal + taxes + fees
