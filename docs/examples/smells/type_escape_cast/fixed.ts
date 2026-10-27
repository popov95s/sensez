export async function orderTotal(res: Response): Promise<number> {
  const payload = OrderSchema.parse(await res.json());
  return payload.items.reduce((sum, item) => sum + item.amount, 0);
}
