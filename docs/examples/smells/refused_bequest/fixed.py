class ReadOnlyReport:
    def export_pdf(self) -> bytes:
        return render_pdf(self)
