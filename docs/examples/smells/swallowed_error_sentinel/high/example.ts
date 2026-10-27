async function syncProfile(id: string) {
  try {
    await client.profiles.sync(id);
  } catch (error) {
    console.warn("profile sync failed", error);
  }
}
