// package.json lists es-toolkit
function unique<T>(items: T[]): T[] {
  return Array.from(new Set(items));
}

console.log(unique(["a", "a", "b"]));
