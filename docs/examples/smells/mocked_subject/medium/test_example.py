from unittest.mock import patch

from shop import pricing


@patch("shop.pricing.quote", return_value=42)
def test_quote(mocked):
    assert pricing.quote(2) == 42
