function publishInvoice(
  invoiceId: string,
  emailCustomer: boolean,
  archivePdf: boolean,
  notifyOwner: boolean,
): void {
  const invoice = repo.load(invoiceId);
  if (emailCustomer) {
    mailer.send(invoice);
  } else {
    reviewQueue.add(invoice);
  }
  if (archivePdf) {
    archive.store(renderPdf(invoice));
  }
  if (notifyOwner) {
    notifier.ping(invoice.owner);
  }
}
