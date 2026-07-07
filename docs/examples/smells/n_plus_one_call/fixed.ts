async function loadProfiles(api: ProfileApi, users: User[]): Promise<Profile[]> {
  const ids = users.map((user) => user.id);
  const profiles = await api.fetchMany(ids);
  return ids.map((id) => profiles[id]);
}
