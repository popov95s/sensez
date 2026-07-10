EmailAddress = str


def notify_all(addresses: list[EmailAddress]) -> None:
    for address in addresses:
        mailer.send(address)
