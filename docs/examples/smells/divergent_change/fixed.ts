class InvoiceRenderer {
  renderHeader(invoice: Invoice): string {
    return invoice.title.toUpperCase();
  }
}

class InvoiceTotals {
  calculateTotal(invoice: Invoice): number {
    return invoice.subtotal + invoice.tax;
  }
}

class InvoiceArchive {
  save(invoice: Invoice): void {
    archive.write(invoice.path);
  }
}
