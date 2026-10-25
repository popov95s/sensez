export async function getOrCreateInvoice(db: Database, invoiceId: string): Promise<Invoice> {
  const invoice = await db.invoices.find(invoiceId);
  if (invoice) {
    return invoice;
  }
  return db.invoices.insert({ id: invoiceId, status: "draft" });
}
