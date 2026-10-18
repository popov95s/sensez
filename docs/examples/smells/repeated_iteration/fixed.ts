function summarize(items: Items): Summary {
  let paid = 0;
  let total = 0;
  for (const item of items) {
    if (item.paid) {
      paid += 1;
    }
    total += item.price;
  }
  return { paid, total };
}
