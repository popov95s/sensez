class Account:
    balance: int
    fee_rate: int
    limit: int

    def billing_fee(self) -> int:
        return self.balance + self.fee_rate + self.limit


class BillingPolicy:
    name: str

    def fee_for(self, account: Account) -> tuple[str, int]:
        return self.name, account.billing_fee()
