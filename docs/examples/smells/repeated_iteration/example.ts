function summarize(items: Items): Summary {
  const paid = items.filter((item) => item.paid).length;
  const total = items.reduce((sum, item) => sum + item.price, 0);
  return { paid, total };
}
