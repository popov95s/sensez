interface Address {
  street: string;
  city: string;
  zipCode: string;
}

function formatLabel(address: Address): string {
  return `${address.street}, ${address.city} ${address.zipCode}`;
}

function shippingZone(address: Address): string {
  return zones.lookup(address.street, address.city, address.zipCode);
}
