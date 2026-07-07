from boundaries.services.queue import enqueue


def handle_request(request: Request) -> Result:
    return enqueue(request.payload)
