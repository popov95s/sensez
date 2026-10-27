class RateTable:
    def __init__(self) -> None:
        self.source: RateSource | None = None

    def get_source(self) -> RateSource:
        if self.source is None:
            self.source = RateSource()
        return self.source
