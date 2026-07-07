class InvoiceReport {
  renderHeader(): string {
    return this.title.toUpperCase();
  }

  calculateTotal(): number {
    return this.subtotal + this.tax;
  }

  saveArchive(): void {
    archive.write(this.path);
  }
}
