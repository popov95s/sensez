class Report {
  exportPdf(): Uint8Array {
    return renderPdf(this);
  }

  sendEmail(): void {
    mailer.send(this);
  }
}

class ReadOnlyReport extends Report {
  exportPdf(): Uint8Array {
    throw new Error("not supported");
  }

  sendEmail(): void {
    throw new Error("not supported");
  }
}
