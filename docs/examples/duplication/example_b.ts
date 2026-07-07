function collectEmails(members: Member[]): string[] {
  const emails: string[] = [];
  for (const member of members) {
    if (member.isActive) {
      emails.push(member.email);
    }
    if (member.isVerified) {
      emails.push(member.email);
    }
  }
  return emails;
}