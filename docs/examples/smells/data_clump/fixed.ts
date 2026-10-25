interface Address {
  street: string;
  city: string;
  zipCode: string;
  country: string;
}

function formatLabel(address: Address): string {
  return `${address.street}, ${address.city} ${address.zipCode}, ${address.country}`;
}

function shippingZone(address: Address): string {
  return zones.lookup(address);
}

function isDeliverable(address: Address): boolean {
  return carriers.covers(address);
}
