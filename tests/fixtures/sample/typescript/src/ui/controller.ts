import { placeOrder } from '../domain/orders';

export function submitOrder(id: string): string {
  const result = placeOrder({ id, preview: false });
  return result.status;
}
