def publish(message):
    if message is None:
        return
    prepare(message)
    if message is None:
        return
    send(message)
