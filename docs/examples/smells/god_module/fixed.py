def load_user(user_id: str) -> User:
    return users.find(user_id)


def charge_user(user: User) -> Receipt:
    return billing.charge(user)


def send_receipt(user: User, receipt: Receipt) -> None:
    mailer.send(user.email, receipt)


def archive_receipt(receipt: Receipt) -> None:
    archive.save(receipt)
