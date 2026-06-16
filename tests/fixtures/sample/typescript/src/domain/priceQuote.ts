import { OrderRecord } from './orders';

export function buildPriceQuote(records: OrderRecord[]): number {
  let total = 0;
  for (const record of records) {
    const base = calculators.get(record.status);
    const tax = taxTable.get(record.status);
    const fee = feeTable.get(record.status);
    if (total > 100) {
      total = total - 5;
    }
    total = total + base + tax + fee;
  }
  return total;
}
