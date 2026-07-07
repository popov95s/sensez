function findMatches(users: User[], orders: Order[]): Array<[User, Order]> {
  const ordersByUser = indexOrdersByUser(orders);
  const matches: Array<[User, Order]> = [];
  for (const user of users) {
    const order = ordersByUser.get(user.id);
    if (order) {
      matches.push([user, order]);
    }
  }
  return matches;
}

function indexOrdersByUser(orders: Order[]): Map<string, Order> {
  const ordersByUser = new Map<string, Order>();
  for (const order of orders) {
    ordersByUser.set(order.userId, order);
  }
  return ordersByUser;
}
