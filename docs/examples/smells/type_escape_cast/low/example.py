from typing import Any, cast


def order_total(raw: object) -> int:
    payload = cast(Any, raw)
    return sum(item["amount"] for item in payload["items"])
