class Account {
  balance = 0;
  feeRate = 0;
  limit = 0;
}

class BillingPolicy {
  name = "standard";

  feeFor(account: Account): [string, number] {
    const fee = account.balance + account.feeRate + account.limit;
    return [this.name, fee];
  }
}
