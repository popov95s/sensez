def sync_profile(profile_id):
    try:
        client.profiles.sync(profile_id)
    except Exception as error:
        log.warning("profile sync failed: %s", error)
