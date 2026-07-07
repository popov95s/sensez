class ReportBuilder {
  build(): Report {
    return buildReport();
  }
}

class ReportMailer {
  send(report: Report): void {
    mailer.send(report);
  }
}

class ReportArchive {
  save(report: Report): void {
    archive.save(report);
  }
}

class ReportBilling {
  charge(report: Report): Receipt {
    return billing.charge(report);
  }
}
