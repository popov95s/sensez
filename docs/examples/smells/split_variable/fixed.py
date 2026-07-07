def invoice_total(invoice: Invoice) -> float:
    subtotal = sum(item.price for item in invoice.items)
    discounted_total = subtotal - invoice.discount
    return discounted_total + invoice.tax
