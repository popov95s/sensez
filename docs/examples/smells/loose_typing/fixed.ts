interface User {
  email: string;
  active: boolean;
}

function notify(user: User): void {
  if (user.active) {
    mailer.send(user.email);
  }
}
