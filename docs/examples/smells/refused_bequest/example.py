class Report:
    def export_pdf(self) -> bytes:
        return render_pdf(self)

    def send_email(self) -> None:
        mailer.send(self)


class ReadOnlyReport(Report):
    def export_pdf(self) -> bytes:
        raise NotImplementedError

    def send_email(self) -> None:
        raise NotImplementedError
