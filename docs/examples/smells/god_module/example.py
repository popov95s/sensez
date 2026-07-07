from god_module.dep_a import run as load_user
from god_module.dep_b import run as charge_user
from god_module.dep_c import run as send_receipt
from god_module.dep_d import run as archive_receipt


def process_checkout() -> int:
    return load_user() + charge_user() + send_receipt() + archive_receipt()
