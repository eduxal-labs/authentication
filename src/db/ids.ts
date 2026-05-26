export const USER_ID_PREFIX = "usr_";

export function generateUserId(): string {
  const id = crypto.randomUUID().replace(/-/g, "");
  return USER_ID_PREFIX + id.slice(0, 16);
}

export function generateVerificationCode(): string {
  const code = crypto.getRandomValues(new Uint8Array(3));
  const num = (code[0] << 16) | (code[1] << 8) | code[2];
  return String(num % 1_000_000).padStart(6, "0");
}
