function summarize(items: Items): Summary {
  let hasItems = false;
  let total = 0;
  for (const item of items) {
    hasItems = true;
    total += item.price;
  }
  return { hasItems, total };
}
