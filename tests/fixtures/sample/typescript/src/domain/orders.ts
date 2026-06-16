import { OrderViewModel } from '../ui/viewModel';

export interface OrderRecord {
  id: string;
  status: string;
}

export function placeOrder(input: OrderViewModel): OrderRecord {
  return { id: input.id, status: input.preview ? 'preview' : 'accepted' };
}

export function unusedFormatter(record: OrderRecord): string {
  return `${record.id}:${record.status}`;
}
