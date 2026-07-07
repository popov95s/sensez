class Account:
    def __init__(self) -> None:
        self._token = "secret"
        self._salt = "pepper"

    def audit_key(self) -> str:
        return self._token + self._salt


def audit_account() -> str:
    account = Account()
    return account.audit_key()
