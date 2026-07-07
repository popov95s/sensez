class Account {
  balance = 0;
  feeRate = 0;
  limit = 0;

  billingFee(): number {
    return this.balance + this.feeRate + this.limit;
  }
}

class BillingPolicy {
  name = "standard";

  feeFor(account: Account): [string, number] {
    return [this.name, account.billingFee()];
  }
}
