function importOrders(rows: Rows): Orders {
  const orders = validOrders(rows);
  const enriched = orders.map(enrich);
  persistImport(enriched);
  return enriched;
}

function validOrders(rows: Rows): Orders {
  const parsed = rows.map(parseOrder);
  return parsed.filter((order) => order.isValid);
}

function persistImport(orders: Orders): void {
  saveOrders(orders);
  notifyImportComplete(orders);
  archiveTotals(orders.map(calculateTotal));
}
