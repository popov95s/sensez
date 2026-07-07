class InvoiceReport:
    def render_header(self) -> str:
        return self.title.upper()

    def calculate_total(self) -> float:
        return self.subtotal + self.tax

    def save_archive(self) -> None:
        archive.write(self.path)
