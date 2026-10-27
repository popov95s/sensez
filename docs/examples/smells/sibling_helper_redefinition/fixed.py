import logging

from example import message_of


def log_failure(failure):
    logging.error(message_of(failure))
