MAX_RETRIES = 7


def is_retryable(attempts: int) -> bool:
    return attempts < MAX_RETRIES
