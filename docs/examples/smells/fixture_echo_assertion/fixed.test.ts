import assert from "node:assert/strict";
import test from "node:test";

function rename(label: string): string {
  return label.trim();
}

test("keeps the label", () => {
  assert.equal(rename(" box "), "box");
});
