class TaxPolicy {
  constructor(private readonly rate: number) {}

  calculate(amount: number): number {
    return amount * this.rate;
  }
}
