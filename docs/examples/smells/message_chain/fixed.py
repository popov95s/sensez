class Order:
    def shipping_country(self) -> str:
        return self.customer.shipping_country()


class Customer:
    def shipping_country(self) -> str:
        return self.address.country_name()


class Address:
    def country_name(self) -> str:
        return self.city.country_name()


class City:
    def country_name(self) -> str:
        return self.country.name


def country(order: Order) -> str:
    return order.shipping_country()
