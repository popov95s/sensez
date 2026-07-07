function loadUser(userId: string): User {
  return users.find(userId);
}

function chargeUser(user: User): Receipt {
  return billing.charge(user);
}

function sendReceipt(user: User, receipt: Receipt): void {
  mailer.send(user.email, receipt);
}

function archiveReceipt(receipt: Receipt): void {
  archive.save(receipt);
}
