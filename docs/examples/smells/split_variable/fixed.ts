function invoiceTotal(invoice: Invoice): number {
  const subtotal = sum(invoice.items.map((item) => item.price));
  const discountedTotal = subtotal - invoice.discount;
  return discountedTotal + invoice.tax;
}
