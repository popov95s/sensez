from typing import Any


def notify(user: dict[str, Any]) -> None:
    if user["active"]:
        mailer.send(user["email"])
