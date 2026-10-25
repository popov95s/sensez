from unittest.mock import patch

from shop.pricing import Pricing


def test_quote():
    with patch.object(Pricing, "rate", return_value=2):
        assert Pricing().quote(2) == 4
