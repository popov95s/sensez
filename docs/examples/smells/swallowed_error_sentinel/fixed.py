def load_profile(profile_id):
    try:
        return client.profiles.get(profile_id)
    except ClientError as error:
        raise ProfileUnavailable(profile_id) from error
