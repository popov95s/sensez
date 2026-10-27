function formatLabel(street: string, city: string, zipCode: string, country: string): string {
  return `${street}, ${city} ${zipCode}, ${country}`;
}

function shippingZone(street: string, city: string, zipCode: string, country: string): string {
  return zones.lookup(street, city, zipCode, country);
}

function isDeliverable(street: string, city: string, zipCode: string, country: string): boolean {
  return carriers.covers(street, city, zipCode, country);
}
