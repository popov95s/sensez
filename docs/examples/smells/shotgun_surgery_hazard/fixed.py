class TaxPolicy:
    def __init__(self, rate: float) -> None:
        self.rate = rate

    def calculate(self, amount: float) -> float:
        return amount * self.rate
