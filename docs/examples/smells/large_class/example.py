class ReportManager:
    def build(self) -> Report:
        return build_report()

    def send(self) -> None:
        mailer.send()

    def archive(self) -> None:
        archive.save()

    def bill(self) -> Receipt:
        return billing.charge()
