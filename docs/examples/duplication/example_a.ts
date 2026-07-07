function collectEmails(recipients: Recipient[]): string[] {
  const emails: string[] = [];
  for (const recipient of recipients) {
    if (recipient.isActive) {
      emails.push(recipient.email);
    }
    if (recipient.isVerified) {
      emails.push(recipient.email);
    }
  }
  return emails;
}