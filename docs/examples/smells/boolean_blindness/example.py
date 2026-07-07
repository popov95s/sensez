def publish_invoice(
    invoice_id: str,
    email_customer: bool,
    archive_pdf: bool,
) -> None:
    invoice = repo.load(invoice_id)
    if email_customer:
        mailer.send(invoice)
    else:
        review_queue.add(invoice)
    if archive_pdf:
        archive.store(render_pdf(invoice))
