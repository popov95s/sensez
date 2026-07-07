import { run as loadUser } from "./dep_a";
import { run as chargeUser } from "./dep_b";
import { run as sendReceipt } from "./dep_c";
import { run as archiveReceipt } from "./dep_d";

export function processCheckout(): number {
  return loadUser() + chargeUser() + sendReceipt() + archiveReceipt();
}
