function summarize(items: Items): Summary {
  const hasItems = items.some(Boolean);
  const total = items.reduce((sum, item) => sum + item.price, 0);
  return { hasItems, total };
}
