class ReportManager {
  build(): Report {
    return buildReport();
  }

  send(): void {
    mailer.send();
  }

  archive(): void {
    archive.save();
  }

  bill(): Receipt {
    return billing.charge();
  }
}
