class InvoiceRenderer:
    def render_header(self, invoice: Invoice) -> str:
        return invoice.title.upper()


class InvoiceTotals:
    def calculate_total(self, invoice: Invoice) -> float:
        return invoice.subtotal + invoice.tax


class InvoiceArchive:
    def save(self, invoice: Invoice) -> None:
        archive.write(invoice.path)
