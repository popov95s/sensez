class ReportBuilder:
    def build(self) -> Report:
        return build_report()


class ReportMailer:
    def send(self, report: Report) -> None:
        mailer.send(report)


class ReportArchive:
    def save(self, report: Report) -> None:
        archive.save(report)


class ReportBilling:
    def charge(self, report: Report) -> Receipt:
        return billing.charge(report)
