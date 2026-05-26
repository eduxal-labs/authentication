import type { VerificationCode } from "../types";

const KV_PREFIX = "verify:";
const CHANGE_PHONE_PREFIX = "change-phone:";
const VERIFICATION_TTL = 900; // 15 minutes in seconds

export function verificationKey(phone: string): string {
  return `${KV_PREFIX}${phone}`;
}

export function changePhoneKey(userId: string, newPhone: string): string {
  return `${CHANGE_PHONE_PREFIX}${userId}:${newPhone}`;
}

export async function storeVerificationCode(
  kv: KVNamespace,
  phone: string,
  code: string,
  purpose: "verification" | "change-phone" = "verification",
  extra?: { userId?: string; newPhone?: string }
): Promise<void> {
  const data: VerificationCode = { phone, code, purpose, ...extra };
  const key =
    purpose === "change-phone" && extra?.userId && extra?.newPhone
      ? changePhoneKey(extra.userId, extra.newPhone)
      : verificationKey(phone);

  await kv.put(key, JSON.stringify(data), { expirationTtl: VERIFICATION_TTL });
}

export async function getAndDeleteVerificationCode(
  kv: KVNamespace,
  phone: string
): Promise<VerificationCode | null> {
  const key = verificationKey(phone);
  const raw = await kv.get(key);
  if (!raw) return null;
  await kv.delete(key);
  return JSON.parse(raw) as VerificationCode;
}

export async function getAndDeleteChangePhoneCode(
  kv: KVNamespace,
  userId: string,
  newPhone: string
): Promise<VerificationCode | null> {
  const key = changePhoneKey(userId, newPhone);
  const raw = await kv.get(key);
  if (!raw) return null;
  await kv.delete(key);
  return JSON.parse(raw) as VerificationCode;
}
