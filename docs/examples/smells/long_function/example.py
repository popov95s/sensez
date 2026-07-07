def import_orders(rows: Rows) -> Orders:
    parsed = [parse_order(row) for row in rows]
    valid = [order for order in parsed if order.is_valid]
    enriched = [enrich(order) for order in valid]
    totals = [calculate_total(order) for order in enriched]
    discounts = [calculate_discount(order) for order in enriched]
    taxes = [calculate_tax(order) for order in enriched]
    save_orders(enriched)
    notify_import_complete(enriched)
    archive_totals(totals)
    archive_discounts(discounts)
    archive_taxes(taxes)
    return enriched
