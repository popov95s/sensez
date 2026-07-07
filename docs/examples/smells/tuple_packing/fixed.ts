interface UserSummary {
  name: string;
  email: string;
  plan: string;
}

function summarize(user: User): UserSummary {
  return { name: user.name, email: user.email, plan: user.plan };
}
