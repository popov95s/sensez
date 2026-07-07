from enum import Enum


class InvoiceStatus(Enum):
    DRAFT = "draft"
    PAID = "paid"
    VOID = "void"


def is_allowed(status: InvoiceStatus) -> bool:
    return status in {
        InvoiceStatus.DRAFT,
        InvoiceStatus.PAID,
        InvoiceStatus.VOID,
    }
