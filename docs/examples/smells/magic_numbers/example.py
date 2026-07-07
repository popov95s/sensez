def is_retryable(attempts: int) -> bool:
    return attempts < 7
