function loadProfiles(api: ProfileApi, users: User[]): Profile[] {
  const profiles = [];
  for (const user of users) {
    profiles.push(api.fetch(user.id));
  }
  return profiles;
}
