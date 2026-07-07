def format_label(street: str, city: str, zip_code: str) -> str:
    return f"{street}, {city} {zip_code}"


def shipping_zone(street: str, city: str, zip_code: str) -> str:
    return zones.lookup(street, city, zip_code)
