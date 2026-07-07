function collectEmails(members: Member[]): string[] {
  return members
    .filter((member) => member.isActive || member.isVerified)
    .map((member) => member.email);
}