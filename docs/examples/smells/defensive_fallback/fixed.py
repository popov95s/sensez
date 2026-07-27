def load_payload(raw):
    try:
        return Payload.parse(raw)
    except InvalidPayload as error:
        raise UserInputError() from error
