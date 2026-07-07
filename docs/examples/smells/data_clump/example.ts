function formatLabel(street: string, city: string, zipCode: string): string {
  return `${street}, ${city} ${zipCode}`;
}

function shippingZone(street: string, city: string, zipCode: string): string {
  return zones.lookup(street, city, zipCode);
}
