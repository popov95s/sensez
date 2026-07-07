def load_profiles(api: ProfileApi, users: list[User]) -> list[Profile]:
    profiles = []
    for user in users:
        profiles.append(api.fetch(user.id))
    return profiles
