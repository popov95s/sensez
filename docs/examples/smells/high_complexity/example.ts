function chooseRefund(order: Order): RefundDecision {
  if (order.cancelled) {
    return denyRefund(order, "cancelled");
  }
  if (order.fraudReview) {
    return manualReview(order);
  }
  if (order.daysSincePurchase > 30) {
    return denyRefund(order, "expired");
  }
  if (order.damaged) {
    return fullRefund(order);
  }
  if (order.missingItems) {
    return partialRefund(order);
  }
  if (order.vipCustomer) {
    return goodwillCredit(order);
  }
  return denyRefund(order, "not_eligible");
}
