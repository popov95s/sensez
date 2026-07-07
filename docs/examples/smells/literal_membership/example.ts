function isAllowed(status: string): boolean {
  return ["draft", "paid", "void"].includes(status);
}
