async function loadProfile(id: string): Promise<Profile | null> {
  try {
    return await client.profiles.get(id);
  } catch (error) {
    console.warn("profile load failed", error);
    return null;
  }
}
