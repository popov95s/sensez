def import_orders(rows: Rows) -> Orders:
    orders = _valid_orders(rows)
    enriched = [enrich(order) for order in orders]
    _persist_import(enriched)
    return enriched


def _valid_orders(rows: Rows) -> Orders:
    parsed = [parse_order(row) for row in rows]
    return [order for order in parsed if order.is_valid]


def _persist_import(orders: Orders) -> None:
    save_orders(orders)
    notify_import_complete(orders)
    archive_totals([calculate_total(order) for order in orders])
