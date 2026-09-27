import test from "node:test";

function save(value: number): number {
  return value;
}

test("saves value", () => {
  save(1);
});
