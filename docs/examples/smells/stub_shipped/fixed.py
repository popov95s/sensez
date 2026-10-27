def export_ledger(org_id: str) -> bytes:
    return f"ledger,{org_id}\n".encode()


report = export_ledger("acme")
