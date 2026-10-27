export async function orderTotal(res: Response): Promise<number> {
  const payload = (await res.json()) as any;
  return payload.items.reduce((sum: number, item: { amount: number }) => sum + item.amount, 0);
}
