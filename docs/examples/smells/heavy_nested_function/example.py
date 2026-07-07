def import_customers(rows: list[dict[str, str]]) -> list[Customer]:
    def normalize(row: dict[str, str]) -> Customer:
        email = row["email"].strip().lower()
        status = row["status"].strip()
        plan = row["plan"].strip()
        region = row["region"].strip()
        return Customer(email, status, plan, region)

    return [normalize(row) for row in rows]
