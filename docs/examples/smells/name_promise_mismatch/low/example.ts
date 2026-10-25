export async function getInvoice(db: Database, invoiceId: string): Promise<Invoice | null> {
  const invoice = await db.invoices.find(invoiceId);
  if (!invoice) {
    await db.invoices.insert({ id: invoiceId, status: "draft" });
  }
  return invoice;
}
