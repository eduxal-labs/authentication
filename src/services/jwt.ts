import { SignJWT, jwtVerify } from "jose";
import type { JwtPayload } from "../types";

const TOKEN_TTL_TEMP = "5m";
const TOKEN_TTL_PERMANENT = "30d";

function getSecret(env: { JWT_SECRET: string }): Uint8Array {
  return new TextEncoder().encode(env.JWT_SECRET);
}

export async function createTempToken(
  env: { JWT_SECRET: string },
  phone: string,
): Promise<string> {
  const secret = getSecret(env);
  return new SignJWT({ sub: null, phone, purpose: "registration" } as any)
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt()
    .setExpirationTime(TOKEN_TTL_TEMP)
    .sign(secret);
}

export async function createPermanentToken(
  env: { JWT_SECRET: string },
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
  env: { JWT_SECRET: string },
  token: string,
): Promise<JwtPayload> {
  const secret = getSecret(env);
  const { payload } = await jwtVerify(token, secret);
  return payload as unknown as JwtPayload;
}
