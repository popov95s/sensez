class Account:
    def __init__(self) -> None:
        self._token = "secret"
        self._salt = "pepper"


def audit_account() -> str:
    account: Account = Account()
    return account._token + account._salt
