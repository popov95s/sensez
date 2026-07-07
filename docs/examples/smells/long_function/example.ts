function importOrders(rows: Rows): Orders {
  const parsed = rows.map(parseOrder);
  const valid = parsed.filter((order) => order.isValid);
  const enriched = valid.map(enrich);
  const totals = enriched.map(calculateTotal);
  const discounts = enriched.map(calculateDiscount);
  const taxes = enriched.map(calculateTax);
  saveOrders(enriched);
  notifyImportComplete(enriched);
  archiveTotals(totals);
  archiveDiscounts(discounts);
  archiveTaxes(taxes);
  return enriched;
}
