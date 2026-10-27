interface UserPayload {
  email: string;
  plan: string;
  active: boolean;
  seats: number;
}

function createUser(payload: UserPayload): User {
  return users.create(payload);
}
