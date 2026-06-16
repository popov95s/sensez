export function normalizeOptions(
  options: Record<string, any>,
  sendEmail: boolean,
  dryRun: boolean,
  force: boolean,
): [string, number, boolean] {
  options["normalized"] = true;
  const phase = options["phase"];
  const channel = options["channel"];
  const region = options["region"];
  const retries = options["retries"];
  if (["new", "retry", "queued"].includes(phase)) {
    options["phase"] = "scheduled";
  }
  return [phase + channel + region, Number(retries), sendEmail || dryRun || force];
}

export function deeplyNested(flagA: boolean, flagB: boolean, flagC: boolean, flagD: boolean): string {
  if (flagA) {
    if (flagB) {
      if (flagC) {
        if (flagD) {
          return "deep";
        }
      }
    }
  }
  return "flat";
}
