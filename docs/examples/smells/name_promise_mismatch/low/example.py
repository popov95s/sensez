def get_invoice(db: Database, invoice_id: str) -> Invoice | None:
    invoice = db.invoices.find(invoice_id)
    if invoice is None:
        db.invoices.insert({"id": invoice_id, "status": "draft"})
    return invoice
