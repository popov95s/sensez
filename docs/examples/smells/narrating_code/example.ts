function activatePlan(account: Account): void {
  // Load the account owner.
  const owner = account.owner;
  // Load the billing profile.
  const billing = owner.billingProfile;
  // Check whether billing is enabled.
  if (!billing.enabled) {
    return;
  }
  // Load the selected plan.
  const plan = billing.selectedPlan;
  // Activate the selected plan.
  account.activate(plan);
  // Notify the owner.
  notifyOwner(owner, plan);
}
