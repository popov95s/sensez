function normalize(value: number): number {
  return value + 1;
}

function middle(value: number): number {
  return normalize(value);
}

function outer(value: number): number {
  return middle(value);
}

const result = outer(2);
console.log(result);
