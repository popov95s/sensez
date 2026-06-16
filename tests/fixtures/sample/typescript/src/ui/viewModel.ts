import { OrderRecord } from '../domain/orders';

export interface OrderViewModel {
  id: string;
  preview: boolean;
}

export function renderOrder(record: OrderRecord): string {
  return record.status.toUpperCase();
}
