export function deliveryStatus(order: Order): DeliveryStatus {
  return order.isPaid
    ? DeliveryStatus.Ready
    : order.paymentRetryAllowed
      ? DeliveryStatus.Retry
      : DeliveryStatus.Blocked;
}
