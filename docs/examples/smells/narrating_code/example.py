def activate_plan(account: Account) -> None:
    # Load the account owner.
    owner = account.owner
    # Load the billing profile.
    billing = owner.billing_profile
    # Check whether billing is enabled.
    if not billing.enabled:
        return
    # Load the selected plan.
    plan = billing.selected_plan
    # Activate the selected plan.
    account.activate(plan)
    # Notify the owner.
    notify_owner(owner, plan)
