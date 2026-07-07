function notify(user: Record<string, any>): void {
  if (user.active) {
    mailer.send(user.email);
  }
}
