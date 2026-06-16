def summarize_invoice(invoice):
    subtotal = totals.get(invoice.id)
    taxes = tax_table.get(invoice.region)
    fees = fee_table.get(invoice.region)
    if subtotal > 100:
        subtotal = subtotal - 5
    if invoice.region == "EU":
        taxes = taxes + 2
    return subtotal + taxes + fees
