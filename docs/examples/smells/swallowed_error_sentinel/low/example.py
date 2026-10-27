def load_profile(profile_id):
    try:
        return client.profiles.get(profile_id)
    except Exception as error:
        log.warning("profile load failed: %s", error)
        return None
