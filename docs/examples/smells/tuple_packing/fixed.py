from dataclasses import dataclass


@dataclass(frozen=True)
class UserSummary:
    name: str
    email: str
    seats: int


def summarize(user: User) -> UserSummary:
    return UserSummary(name=user.name, email=user.email, seats=user.seats)
