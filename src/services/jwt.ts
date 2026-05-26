import { SignJWT, jwtVerify } from "jose";
import type { JwtPayload } from "../types";

const TOKEN_TTL_TEMP = "5m";
const TOKEN_TTL_PERMANENT = "30d";

function getSecret(env: { PASETO_PASSWORD: string }): Uint8Array {
  return new TextEncoder().encode(env.PASETO_PASSWORD);
}

export async function createTempToken(
  env: { PASETO_PASSWORD: string },
  phone: string,
): Promise<string> {
  const secret = getSecret(env);
  return new SignJWT({ phone, purpose: "registration" } as Record<
    string,
    unknown
  >)
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt()
    .setExpirationTime(TOKEN_TTL_TEMP)
    .sign(secret);
}

export async function createPermanentToken(
  env: { PASETO_PASSWORD: string },
  userId: string,
  phone: string,
): Promise<string> {
  const secret = getSecret(env);
  return new SignJWT({ sub: userId, phone, purpose: "auth" })
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt()
    .setExpirationTime(TOKEN_TTL_PERMANENT)
    .sign(secret);
}

export async function verifyToken(
  env: { PASETO_PASSWORD: string },
  token: string,
): Promise<JwtPayload> {
  const secret = getSecret(env);
  const { payload } = await jwtVerify(token, secret);
  return payload as unknown as JwtPayload;
}
