type EmailAddress = string;

function notifyAll(addresses: EmailAddress[]): void {
  for (const address of addresses) {
    mailer.send(address);
  }
}
