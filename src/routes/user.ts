import { Hono } from "hono";
import { requireAuth } from "../middleware/auth";
import { userQueries } from "../db/queries";
import { generateVerificationCode } from "../db/ids";
import {
  storeVerificationCode,
  getAndDeleteChangePhoneCode,
} from "../services/kv";
import { sendWhatsAppVerification } from "../services/whatsapp";
import { createPermanentToken } from "../services/jwt";
import type {
  UpdateProfileRequest,
  ChangePhoneRequest,
  ChangePhoneVerifyRequest,
  JwtPayload,
} from "../types";

type Bindings = {
  VERIFICATION_KV: KVNamespace;
  DB: D1Database;
  AVATARS_BUCKET: R2Bucket;
  PASETO_PASSWORD: string;
  WHATSAPP_PHONE_NUMBER_ID: string;
  WHATSAPP_TOKEN: string;
};

const user = new Hono<{
  Bindings: Bindings;
  Variables: { jwtPayload: JwtPayload };
}>();

// All routes under /user require permanent JWT auth
user.use("*", requireAuth);

/**
 * GET /user/me
 * Returns the authenticated user's full profile.
 */
user.get("/me", async (c) => {
  const { jwtPayload } = c.var;
  const db = userQueries(c.env.DB);

  const currentUser = await db.findById(jwtPayload.sub!);
  if (!currentUser) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  return c.json({ user: currentUser });
});

/**
 * PUT /user/me
 * Updates the authenticated user's profile (name and/or avatar).
 */
user.put("/me", async (c) => {
  const { jwtPayload } = c.var;
  const db = userQueries(c.env.DB);

  const currentUser = await db.findById(jwtPayload.sub!);
  if (!currentUser) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  const formData = await c.req.parseBody();
  const nameField = formData["name"] as string | undefined;
  const avatarFile = formData["avatar"] as File | undefined;

  const updates: { name?: string; avatar_url?: string } = {};

  if (nameField !== undefined) {
    if (nameField.trim().length === 0) {
      return c.json(
        { error: "validation", message: "Name cannot be empty" },
        400,
      );
    }
    updates.name = nameField.trim();
  }

  if (avatarFile && avatarFile.size > 0) {
    const ext = avatarFile.name.split(".").pop() ?? "jpg";
    const key = `avatars/${currentUser.id}.${ext}`;

    await c.env.AVATARS_BUCKET.put(key, avatarFile.stream(), {
      httpMetadata: { contentType: avatarFile.type },
    });
    updates.avatar_url = key;
  }

  if (Object.keys(updates).length === 0) {
    return c.json({ error: "validation", message: "No fields to update" }, 400);
  }

  const updated = await db.updateProfile(currentUser.id, updates);
  return c.json({ user: updated });
});

/**
 * POST /user/change-phone/request
 * Initiates phone number change — sends a verification code to the new phone.
 */
user.post("/change-phone/request", async (c) => {
  const { jwtPayload } = c.var;
  const body = await c.req.json<ChangePhoneRequest>();
  const { new_phone } = body;

  if (!new_phone || !new_phone.startsWith("+")) {
    return c.json(
      { error: "validation", message: "New phone must be in E.164 format" },
      400,
    );
  }

  const db = userQueries(c.env.DB);

  // Check no other user has this phone
  const existing = await db.findByPhone(new_phone);
  if (existing && existing.id !== jwtPayload.sub) {
    return c.json(
      { error: "conflict", message: "Phone number is already taken" },
      409,
    );
  }

  const code = generateVerificationCode();
  const userId = jwtPayload.sub!;

  await storeVerificationCode(
    c.env.VERIFICATION_KV,
    new_phone,
    code,
    "change-phone",
    {
      userId,
      newPhone: new_phone,
    },
  );

  await sendWhatsAppVerification(
    c.env.WHATSAPP_PHONE_NUMBER_ID,
    c.env.WHATSAPP_TOKEN,
    new_phone,
    code,
  );

  return c.json({
    success: true,
    message: "Verification code sent to new phone",
  });
});

/**
 * POST /user/change-phone/verify
 * Verifies the code and updates the phone number.
 */
user.post("/change-phone/verify", async (c) => {
  const { jwtPayload } = c.var;
  const body = await c.req.json<ChangePhoneVerifyRequest>();
  const { new_phone, code } = body;

  if (!new_phone || !code) {
    return c.json(
      { error: "validation", message: "New phone and code are required" },
      400,
    );
  }

  const userId = jwtPayload.sub!;

  const verification = await getAndDeleteChangePhoneCode(
    c.env.VERIFICATION_KV,
    userId,
    new_phone,
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

  // Double-check the phone isn't taken (race condition guard)
  const existing = await db.findByPhone(new_phone);
  if (existing && existing.id !== userId) {
    return c.json(
      {
        error: "conflict",
        message: "Phone number was taken before verification completed",
      },
      409,
    );
  }

  const updated = await db.updatePhone(userId, new_phone);
  if (!updated) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  // Issue a new JWT with the updated phone
  const token = await createPermanentToken(c.env, updated.id, updated.phone);

  return c.json({ token, user: updated });
});

/**
 * GET /user/:id
 * Returns a public profile for another user (excludes phone for privacy).
 */
user.get("/:id", async (c) => {
  const targetId = c.req.param("id");
  const db = userQueries(c.env.DB);

  const profile = await db.getPublicProfile(targetId);
  if (!profile) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  return c.json({ user: profile });
});

export default user;
