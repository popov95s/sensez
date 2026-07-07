def invoice_total(invoice: Invoice) -> float:
    result = sum(item.price for item in invoice.items)
    result = result - invoice.discount
    result = result + invoice.tax
    return result
