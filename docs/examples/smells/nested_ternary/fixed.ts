function deliveryStatus(order: Order): DeliveryStatus {
  if (order.isPaid) {
    return DeliveryStatus.Ready;
  }
  if (order.paymentRetryAllowed) {
    return DeliveryStatus.Retry;
  }
  return DeliveryStatus.Blocked;
}

