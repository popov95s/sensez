from app.api.schemas import OrderPayload


class OrderResult:
    def __init__(self, status: str):
        self.status = status


def place_order(payload: OrderPayload) -> OrderResult:
    if payload.raw.get("preview"):
        return OrderResult("preview")
    return OrderResult("accepted")


def stale_discount(amount: int) -> int:
    return amount - 3


def overloaded_policy(kind: str, amount: int, region: str, channel: str, urgent: bool) -> int:
    score = amount
    if kind == "retail":
        score += 1
    if region == "eu":
        score += 2
    if channel == "partner":
        score += 3
    if urgent:
        score += 5
    return score
