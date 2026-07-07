from dataclasses import dataclass


@dataclass(frozen=True)
class Address:
    street: str
    city: str
    zip_code: str


def format_label(address: Address) -> str:
    return f"{address.street}, {address.city} {address.zip_code}"


def shipping_zone(address: Address) -> str:
    return zones.lookup(address.street, address.city, address.zip_code)
