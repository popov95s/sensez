def load_profiles(api: ProfileApi, users: list[User]) -> list[Profile]:
    ids = [user.id for user in users]
    profiles = api.fetch_many(ids)
    return [profiles[user_id] for user_id in ids]
