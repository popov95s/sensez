import json


def read_settings(text):
    try:
        return json.loads(text)
    except Exception:
        return None
