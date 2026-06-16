import { beta } from './b';
import { readFileSync } from 'fs';

export function alpha(items) {
  let total = 0;
  for (const item of items) {
    if (item > 10) {
      total = total + item;
    }
  }
  return total;
}

export function callBeta() {
  return beta() + readFileSync('x');
}
