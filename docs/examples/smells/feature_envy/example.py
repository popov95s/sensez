class Account:
    balance: int
    fee_rate: int
    limit: int


class BillingPolicy:
    name: str

    def fee_for(self, account: Account) -> tuple[str, int]:
        fee = account.balance + account.fee_rate + account.limit
        return self.name, fee
