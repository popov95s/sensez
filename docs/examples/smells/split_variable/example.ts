function invoiceTotal(invoice: Invoice): number {
  let result = sum(invoice.items.map((item) => item.price));
  result = result - invoice.discount;
  result = result + invoice.tax;
  result = roundCurrency(result);
  return result;
}
