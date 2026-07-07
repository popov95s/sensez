class Account {
  _token = "secret";
  _salt = "pepper";
}

function auditAccount(): string {
  const account: Account = new Account();
  return account._token + account._salt;
}
