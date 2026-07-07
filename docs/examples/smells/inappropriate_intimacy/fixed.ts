class Account {
  private token = "secret";
  private salt = "pepper";

  auditKey(): string {
    return this.token + this.salt;
  }
}

function auditAccount(): string {
  const account = new Account();
  return account.auditKey();
}
