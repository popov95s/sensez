def collect_emails(members: Members) -> Emails:
    return [
        member.email for member in members if member.is_active or member.is_verified
    ]
