enum InvoiceStatus {
  Draft = "draft",
  Paid = "paid",
  Void = "void",
}

function isAllowed(status: InvoiceStatus): boolean {
  return status === InvoiceStatus.Draft ||
    status === InvoiceStatus.Paid ||
    status === InvoiceStatus.Void;
}
