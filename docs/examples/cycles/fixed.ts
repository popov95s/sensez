export function g(): number {
  return h() + 1;
}

export function h(): number {
  return base() - 1;
}

function base(): number {
  return 1;
}
