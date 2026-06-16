import { alpha } from './a';

export function beta(values) {
  let sum = 0;
  for (const value of values) {
    if (value > 10) {
      sum = sum + value;
    }
  }
  return sum;
}

export function callAlpha() {
  return alpha([1, 2, 3]);
}
