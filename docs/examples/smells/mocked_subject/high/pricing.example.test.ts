// pricing.example.test.ts: the mocked module shares the test file's name.
import { render } from "./receipt";

vi.mock("./pricing");

it("renders the receipt", () => {
  expect(render({ qty: 2 })).toContain("4");
});
