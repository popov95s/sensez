def collect_emails(recipients: Recipients) -> list[str]:
    emails = []
    for recipient in recipients:
        if recipient.is_active:
            emails.append(recipient.email)
        if recipient.is_verified:
            emails.append(recipient.email)
    return emails
