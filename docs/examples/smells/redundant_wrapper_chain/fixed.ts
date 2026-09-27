function normalize(value: number): number {
  return value + 1;
}

function outer(value: number): number {
  return normalize(value);
}

const result = outer(2);
console.log(result);
