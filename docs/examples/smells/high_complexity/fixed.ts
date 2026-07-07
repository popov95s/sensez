type RefundHandler = (order: Order) => RefundDecision;

const refundPolicy: Record<string, RefundHandler> = {
  cancelled: denyCancelled,
  fraud_review: manualReview,
  expired: denyExpired,
  damaged: fullRefund,
  missing_items: partialRefund,
  vip_customer: goodwillCredit,
};

function chooseRefund(order: Order): RefundDecision {
  const handler = refundPolicy[order.refundReason()] ?? denyNotEligible;
  return handler(order);
}
