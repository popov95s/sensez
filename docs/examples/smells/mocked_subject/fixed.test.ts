import { quote } from "./pricing";

vi.mock("./rates", () => ({ fetchRate: vi.fn(() => 1.5) }));

it("quotes", () => {
  expect(quote({ qty: 2 })).toBe(3);
});
