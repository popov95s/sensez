from typing import Protocol


class InvoiceDelivery(Protocol):
    def deliver(self, invoice: Invoice) -> None: ...


class InvoiceArchival(Protocol):
    def archive(self, invoice: Invoice) -> None: ...


class EmailDelivery:
    def deliver(self, invoice: Invoice) -> None:
        mailer.send(invoice)


class ReviewQueueDelivery:
    def deliver(self, invoice: Invoice) -> None:
        review_queue.add(invoice)


class PdfArchival:
    def archive(self, invoice: Invoice) -> None:
        archive.store(render_pdf(invoice))


class SkipArchival:
    def archive(self, invoice: Invoice) -> None:
        return None


def publish_invoice(
    invoice_id: str,
    delivery: InvoiceDelivery,
    archival: InvoiceArchival,
) -> None:
    invoice = repo.load(invoice_id)
    delivery.deliver(invoice)
    archival.archive(invoice)
