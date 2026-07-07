def normalize_customer(row: Row) -> Customer:
    email = row["email"].strip().lower()
    status = row["status"].strip()
    plan = row["plan"].strip()
    region = row["region"].strip()
    return Customer(email, status, plan, region)


def import_customers(rows: Rows) -> Customers:
    return [normalize_customer(row) for row in rows]
