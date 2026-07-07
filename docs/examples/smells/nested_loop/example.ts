function findMatches(users: User[], orders: Order[]): Array<[User, Order]> {
  const matches: Array<[User, Order]> = [];
  for (const user of users) {
    for (const order of orders) {
      if (order.userId === user.id) {
        matches.push([user, order]);
      }
    }
  }
  return matches;
}
