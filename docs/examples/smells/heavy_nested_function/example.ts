function importCustomers(rows: Row[]): Customer[] {
  function normalize(row: Row): Customer {
    const email = row.email.trim().toLowerCase();
    const status = row.status.trim();
    const plan = row.plan.trim();
    const region = row.region.trim();
    return new Customer(email, status, plan, region);
  }

  return rows.map(normalize);
}
