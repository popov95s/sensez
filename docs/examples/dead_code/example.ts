export function main(): string {
  return process();
}

export function process(): string {
  return "ok";
}

function unusedHelper(): string {
  return "ignored";
}
