function createUser(payload: Record<string, unknown>): User {
  const email = payload["email"];
  const plan = payload["plan"];
  const active = payload["active"];
  const seats = payload["seats"];
  return users.create(email, plan, active, seats);
}
