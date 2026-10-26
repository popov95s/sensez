from typing import Protocol


class InvoiceDelivery(Protocol):
    def deliver(self, invoice: Invoice) -> None: ...


class InvoiceFollowUp(Protocol):
    def run(self, invoice: Invoice) -> None: ...


class EmailDelivery:
    def deliver(self, invoice: Invoice) -> None:
        mailer.send(invoice)


class ReviewQueueDelivery:
    def deliver(self, invoice: Invoice) -> None:
        review_queue.add(invoice)


class PdfArchival:
    def run(self, invoice: Invoice) -> None:
        archive.store(render_pdf(invoice))


class OwnerNotification:
    def run(self, invoice: Invoice) -> None:
        notifier.ping(invoice.owner)


def publish_invoice(
    invoice_id: str,
    delivery: InvoiceDelivery,
    follow_ups: list[InvoiceFollowUp],
) -> None:
    invoice = repo.load(invoice_id)
    delivery.deliver(invoice)
    for follow_up in follow_ups:
        follow_up.run(invoice)
