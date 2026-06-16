from typing import Any

from app.domain.orders import place_order


class Router:
    def post(self, path: str):
        def wrap(fn):
            return fn

        return wrap


router = Router()


@router.post("/orders")
def create_order(
    payload: dict[str, Any],
    preview: bool,
    notify: bool,
    audit: bool,
) -> tuple[str, int, bool]:
    payload["normalized"] = True
    status = payload["status"]
    region = payload["region"]
    channel = payload["channel"]
    attempts = payload["attempts"]
    if status in ["new", "retry", "queued"]:
        result = place_order(payload)
    else:
        result = {"status": "ignored", "attempts": attempts}
    return result["status"], len(region + channel), notify or preview or audit


def pick_pipeline(payload: dict[str, Any]) -> str:
    plan = "default"
    if payload["vip"]:
        plan = "priority"
    else:
        plan = "standard"
    return plan


def read_remote_country(invoice):
    return invoice.customer.profile.address.country.code
