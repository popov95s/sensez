from typing import Any


def notify_contact(raw_contact: Any) -> None:
    if raw_contact["active"]:
        mailer.send(raw_contact["email"])
