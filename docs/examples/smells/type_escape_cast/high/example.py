def primary_email(user: User) -> str:
    return user.primary_email.address  # type: ignore
