def create_user(payload: dict[str, object]) -> User:
    email = payload["email"]
    plan = payload["plan"]
    active = payload["active"]
    return users.create(email, plan, active)
