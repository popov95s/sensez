from dataclasses import dataclass


@dataclass(frozen=True)
class UserContact:
    email: str
    active: bool


def notify(user: UserContact) -> None:
    if user.active:
        mailer.send(user.email)
