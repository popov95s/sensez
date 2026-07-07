def find_matches(users: list[User], orders: list[Order]) -> list[tuple[User, Order]]:
    orders_by_user = {order.user_id: order for order in orders}
    matches = []
    for user in users:
        order = orders_by_user.get(user.id)
        if order is not None:
            matches.append((user, order))
    return matches
