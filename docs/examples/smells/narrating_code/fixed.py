def activate_plan(account: Account) -> None:
    owner = account.owner
    billing = owner.billing_profile
    if not billing.enabled:
        return
    _activate_selected_plan(account, owner, billing)


def _activate_selected_plan(
    account: Account,
    owner: Owner,
    billing: BillingProfile,
) -> None:
    plan = billing.selected_plan
    account.activate(plan)
    notify_owner(owner, plan)
