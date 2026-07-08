function activatePlan(account: Account): void {
  const owner = account.owner;
  const billing = owner.billingProfile;
  if (!billing.enabled) {
    return;
  }
  activateSelectedPlan(account, owner, billing);
}

function activateSelectedPlan(
  account: Account,
  owner: Owner,
  billing: BillingProfile,
): void {
  const plan = billing.selectedPlan;
  account.activate(plan);
  notifyOwner(owner, plan);
}
