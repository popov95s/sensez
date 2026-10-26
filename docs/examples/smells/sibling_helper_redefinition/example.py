def message_of(error):
    return str(error.args[0]) if error.args else repr(error)
