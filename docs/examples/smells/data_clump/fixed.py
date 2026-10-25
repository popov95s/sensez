from dataclasses import dataclass


@dataclass(frozen=True)
class Address:
    street: str
    city: str
    zip_code: str
    country: str


def format_label(address: Address) -> str:
    return f"{address.street}, {address.city} {address.zip_code}, {address.country}"


def shipping_zone(address: Address) -> str:
    return zones.lookup(address)


def is_deliverable(address: Address) -> bool:
    return carriers.covers(address)
