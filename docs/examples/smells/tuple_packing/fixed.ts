interface UserSummary {
  name: string;
  email: string;
  seats: number;
}

function summarize(user: User): UserSummary {
  return { name: user.name, email: user.email, seats: user.seats };
}
