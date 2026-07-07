from dataclasses import dataclass


@dataclass(frozen=True)
class UserSummary:
    name: str
    email: str
    plan: str


def summarize(user: User) -> UserSummary:
    return UserSummary(name=user.name, email=user.email, plan=user.plan)
