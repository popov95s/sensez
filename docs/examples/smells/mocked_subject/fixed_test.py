from shop import pricing


def test_quote(mocker):
    mocker.patch("shop.pricing.fetch_rate", return_value=2)
    assert pricing.quote(2) == 4
