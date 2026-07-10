function notifyContact(rawContact: any): void {
  if (rawContact.active) {
    mailer.send(rawContact.email);
  }
}
