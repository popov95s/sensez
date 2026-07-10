function notifyContact(contact: Record<string, any>): void {
  if (contact.active) {
    mailer.send(contact.email);
  }
}
