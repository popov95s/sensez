from typing import Any


def notify_contact(contact: dict[str, Any]) -> None:
    if contact["active"]:
        mailer.send(contact["email"])
