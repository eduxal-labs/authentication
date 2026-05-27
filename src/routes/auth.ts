import { Hono } from "hono";
import type {
  AuthResponse,
  SendCodeRequest,
  VerifyCodeRequest,
} from "../types";
import { Status } from "../types";
import { Level } from "../types";
import { generateUserId, generateVerificationCode } from "../db/ids";
import { userQueries } from "../db/queries";
import {
  storeVerificationCode,
  getAndDeleteVerificationCode,
} from "../services/kv";
import { sendWhatsAppVerification } from "../services/whatsapp";
import { createTempToken, createPermanentToken } from "../services/jwt";
import { requireTempToken } from "../middleware/auth";

type Bindings = {
  VERIFICATION_KV: KVNamespace;
  DB: D1Database;
  AVATARS_BUCKET: R2Bucket;
  JWT_SECRET: string;
  WHATSAPP_PHONE_NUMBER_ID: string;
  WHATSAPP_TOKEN: string;
};

const auth = new Hono<{ Bindings: Bindings }>();

/**
 * POST /auth/send-code
 * Sends a verification code to the user's phone via WhatsApp.
 */
auth.post("/send-code", async (c) => {
  const body = await c.req.json<SendCodeRequest>();
  const { phone } = body;

  if (!phone || !phone.startsWith("+")) {
    return c.json(
      {
        error: "validation",
        message: "Phone must be in E.164 format (e.g., +252612345678)",
      },
      400,
    );
  }

  const code = generateVerificationCode();

  await storeVerificationCode(
    c.env.VERIFICATION_KV,
    phone,
    code,
    "verification",
  );

  await sendWhatsAppVerification(
    c.env.WHATSAPP_PHONE_NUMBER_ID,
    c.env.WHATSAPP_TOKEN,
    phone,
    code,
  );

  return c.json({ success: true, message: "Verification code sent" });
});

/**
 * POST /auth/verify-code
 * Verifies the code and either authenticates an existing user or issues a temp token for registration.
 */
auth.post("/verify-code", async (c) => {
  const body = await c.req.json<VerifyCodeRequest>();
  const { phone, code } = body;

  if (!phone || !code) {
    return c.json(
      { error: "validation", message: "Phone and code are required" },
      400,
    );
  }

  const verification = await getAndDeleteVerificationCode(
    c.env.VERIFICATION_KV,
    phone,
  );
  if (!verification) {
    return c.json(
      { error: "invalid_code", message: "Code expired or not requested" },
      401,
    );
  }

  if (verification.code !== code) {
    return c.json(
      { error: "invalid_code", message: "Invalid verification code" },
      401,
    );
  }

  const db = userQueries(c.env.DB);
  const user = await db.findByPhone(phone);

  if (user) {
    // Reject suspended users
    if (user.status === Status.Suspended) {
      return c.json(
        { error: "forbidden", message: "Account is suspended" },
        403,
      );
    }

    // Activate Deleted / Invited users on login
    const activeUser = await db.activateIfNeeded(user);

    const token = await createPermanentToken(
      c.env,
      activeUser.id,
      activeUser.phone,
      activeUser.level,
    );
    const response: AuthResponse = {
      status: "existing_user",
      token,
      user: activeUser,
    };
    return c.json(response);
  }

  // New user — issue temporary token
  const tempToken = await createTempToken(c.env, phone);
  const response: AuthResponse = {
    status: "new_user",
    temp_token: tempToken,
    phone,
  };
  return c.json(response);
});

/**
 * POST /auth/register
 * Registers a new user. Requires a temporary registration token.
 * Accepts either JSON or multipart/form-data (with optional avatar file).
 */
auth.post("/register", requireTempToken, async (c) => {
  const { jwtPayload } = c.var;
  const contentType = c.req.header("Content-Type") ?? "";

  let phone: string;
  let name: string;
  let avatarFile: File | undefined;

  if (contentType.includes("multipart/form-data")) {
    const formData = await c.req.parseBody();
    phone = formData["phone"] as string;
    name = formData["name"] as string;
    avatarFile = formData["avatar"] as File | undefined;
  } else {
    const body = await c.req.json<{ phone: string; name: string }>();
    phone = body.phone;
    name = body.name;
  }

  if (!name || name.trim().length === 0) {
    return c.json({ error: "validation", message: "Name is required" }, 400);
  }

  if (!phone || phone !== jwtPayload.phone) {
    return c.json(
      {
        error: "forbidden",
        message: "Phone does not match the verification token",
      },
      403,
    );
  }

  const db = userQueries(c.env.DB);

  // Check if user was already created
  const existing = await db.findByPhone(phone);
  if (existing) {
    const token = await createPermanentToken(
      c.env,
      existing.id,
      existing.phone,
      existing.level,
    );
    return c.json({ token, user: existing });
  }

  const userId = generateUserId();
  let avatarUrl: string | null = null;

  if (avatarFile && avatarFile.size > 0) {
    const ext = avatarFile.name.split(".").pop() ?? "jpg";
    const key = `avatars/${userId}.${ext}`;

    await c.env.AVATARS_BUCKET.put(key, avatarFile.stream(), {
      httpMetadata: { contentType: avatarFile.type },
    });
    avatarUrl = key;
  }

  const now = new Date().toISOString();
  const nowEpoch = Math.floor(Date.now() / 1000);

  const user = {
    id: userId,
    phone,
    name: name.trim(),
    level: Level.Normal,
    status: Status.Active,
    created: nowEpoch,
    avatar_url: avatarUrl,
    created_at: now,
    updated_at: now,
  };

  await db.create(user);

  const token = await createPermanentToken(
    c.env,
    user.id,
    user.phone,
    user.level,
  );
  return c.json({ token, user }, 201);
});

export default auth;
