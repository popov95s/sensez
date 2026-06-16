export interface User {
  id: number;
  name: string;
}

export function makeUser(id: number, name: string): User {
  return { id, name };
}
