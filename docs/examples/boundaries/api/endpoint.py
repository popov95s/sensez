from boundaries.db.client import save


def handle_request(request: Request) -> Result:
    return save(request.payload)
