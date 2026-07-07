const NO_SCORE = 0;
const ENTERPRISE_ORDER_THRESHOLD = 500;
const LARGE_ORDER_THRESHOLD = 100;
const ENTERPRISE_ORDER_SCORE = 25;
const LARGE_ORDER_SCORE = 10;
const STANDARD_ORDER_SCORE = 1;

function score(order: Order): number {
  if (!canScore(order)) {
    return NO_SCORE;
  }
  return scoreTotal(order.total);
}

function canScore(order: Order): boolean {
  if (!order.paid) {
    return false;
  }
  if (!order.customer.active) {
    return false;
  }
  if (order.customer.suspended) {
    return false;
  }
  return true;
}

function scoreTotal(total: number): number {
  if (total >= ENTERPRISE_ORDER_THRESHOLD) {
    return ENTERPRISE_ORDER_SCORE;
  }
  if (total >= LARGE_ORDER_THRESHOLD) {
    return LARGE_ORDER_SCORE;
  }
  return STANDARD_ORDER_SCORE;
}
