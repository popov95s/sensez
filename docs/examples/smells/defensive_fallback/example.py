def load_payload(raw):
    try:
        names = raw.get("names") or []
        options = raw.get("options") or {}
        return names, options
    except:
        return [], {}
