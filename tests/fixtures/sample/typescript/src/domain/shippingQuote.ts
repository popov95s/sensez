import { OrderRecord } from './orders';

export function buildShippingQuote(rows: OrderRecord[]): number {
  let total = 0;
  for (const row of rows) {
    const base = calculators.get(row.status);
    const tax = taxTable.get(row.status);
    const fee = feeTable.get(row.status);
    if (total > 100) {
      total = total - 5;
    }
    total = total + base + tax + fee;
  }
  return total;
}
