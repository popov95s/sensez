def format_label(street: str, city: str, zip_code: str, country: str) -> str:
    return f"{street}, {city} {zip_code}, {country}"


def shipping_zone(street: str, city: str, zip_code: str, country: str) -> str:
    return zones.lookup(street, city, zip_code, country)


def is_deliverable(street: str, city: str, zip_code: str, country: str) -> bool:
    return carriers.covers(street, city, zip_code, country)
