async function loadProfile(id: string): Promise<Result<Profile, LoadError>> {
  try {
    return ok(await client.profiles.get(id));
  } catch (error) {
    return err(toLoadError(error));
  }
}
