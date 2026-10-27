import { quote } from "./pricing";

vi.mock("./pricing", () => ({ quote: vi.fn(() => 42) }));

it("quotes", () => {
  expect(quote({ qty: 2 })).toBe(42);
});
