export const TAX_RATE = 0.2;

export function calculate(amount: number): number {
  return amount * TAX_RATE;
}
