class ReadOnlyReport implements ExportableReport {
  exportPdf(): Uint8Array {
    return renderPdf(this);
  }
}
