from dataclasses import dataclass


@dataclass(frozen=True)
class UserPayload:
    email: str
    plan: str
    active: bool


def create_user(payload: UserPayload) -> User:
    return users.create(payload.email, payload.plan, payload.active)
