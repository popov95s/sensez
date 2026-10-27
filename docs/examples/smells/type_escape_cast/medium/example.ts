export function shippingLabel(order: Order): string {
  const parcel = order as unknown as Parcel;
  return `${parcel.carrier} ${parcel.trackingCode}`;
}
