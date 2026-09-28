import assert from "node:assert/strict";
import test from "node:test";

function save(value: number): number {
  return value;
}

test("saves value", () => {
  assert.equal(save(1), 1);
});
