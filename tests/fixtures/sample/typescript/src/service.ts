import { User, makeUser } from './models';

export function buildUsers(names: string[]): User[] {
  const users: User[] = [];
  for (const name of names) {
    users.push(makeUser(users.length, name));
  }
  return users;
}
