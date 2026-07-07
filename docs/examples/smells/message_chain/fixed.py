class Order:
    def shipping_city(self) -> str:
        return self.customer.shipping_city()


class Customer:
    def shipping_city(self) -> str:
        return self.address.city_name()


class Address:
    def city_name(self) -> str:
        return self.city.name


def city(order: Order) -> str:
    return order.shipping_city()
