const rates = new Map<string, number>();

export function getRate(currency: string): number {
  const known = rates.get(currency);
  if (known !== undefined) {
    return known;
  }
  const rate = currency.length / 10;
  rates.set(currency, rate);
  return rate;
}
