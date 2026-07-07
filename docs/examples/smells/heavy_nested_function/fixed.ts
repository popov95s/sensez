function normalizeCustomer(row: Row): Customer {
  const email = row.email.trim().toLowerCase();
  const status = row.status.trim();
  const plan = row.plan.trim();
  const region = row.region.trim();
  return new Customer(email, status, plan, region);
}

function importCustomers(rows: Row[]): Customer[] {
  return rows.map(normalizeCustomer);
}
