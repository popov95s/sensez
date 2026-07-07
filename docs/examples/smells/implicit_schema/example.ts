function createUser(payload: Record<string, unknown>): User {
  const email = payload["email"];
  const plan = payload["plan"];
  const active = payload["active"];
  return users.create(email, plan, active);
}
