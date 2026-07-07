def collect_emails(members: Members) -> list[str]:
    emails = []
    for member in members:
        if member.is_active:
            emails.append(member.email)
        if member.is_verified:
            emails.append(member.email)
    return emails
