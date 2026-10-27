import logging


def message_of(failure):
    return str(failure.args[0]) if failure.args else repr(failure)


def log_failure(failure):
    logging.error(message_of(failure))
