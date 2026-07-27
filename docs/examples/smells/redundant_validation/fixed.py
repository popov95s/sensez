def publish(message):
    if message is None:
        return
    prepare(message)
    send(message)
