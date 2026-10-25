def get_or_create_invoice(db: Database, invoice_id: str) -> Invoice:
    invoice = db.invoices.find(invoice_id)
    if invoice is not None:
        return invoice
    return db.invoices.insert({"id": invoice_id, "status": "draft"})
