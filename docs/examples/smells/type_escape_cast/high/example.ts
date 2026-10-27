export function primaryEmail(user: User): string {
  return user.emails.find((email) => email.primary)!.address;
}
