def find_matches(users: list[User], orders: list[Order]) -> list[tuple[User, Order]]:
    matches = []
    for user in users:
        for order in orders:
            if order.user_id == user.id:
                matches.append((user, order))
    return matches
