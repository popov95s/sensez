interface UserPayload {
  email: string;
  plan: string;
  active: boolean;
}

function createUser(payload: UserPayload): User {
  return users.create(payload.email, payload.plan, payload.active);
}
