from app.domain.orders import OrderResult


class OrderPayload:
    def __init__(self, raw: dict[str, object]):
        self.raw = raw


def response_model() -> type[OrderResult]:
    return OrderResult
