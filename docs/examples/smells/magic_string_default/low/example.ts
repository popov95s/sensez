async function saveOwner(owner: Owner): Promise<void> {
  await store.save({ label: owner.label ?? "n/a" });
}
