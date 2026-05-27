import { Hono } from "hono";
import { requireAuth, requireSuper } from "../middleware/auth";
import { userQueries } from "../db/queries";
import { generateUserId, generateVerificationCode } from "../db/ids";
import {
  storeVerificationCode,
  getAndDeleteChangePhoneCode,
} from "../services/kv";
import { sendWhatsAppVerification } from "../services/whatsapp";
import { createPermanentToken } from "../services/jwt";
import type {
  InviteRequest,
  UpdateLevelRequest,
  UpdateStatusRequest,
  ChangePhoneRequest,
  ChangePhoneVerifyRequest,
  JwtPayload,
} from "../types";
import { Level, Status } from "../types";

type Bindings = {
  VERIFICATION_KV: KVNamespace;
  DB: D1Database;
  AVATARS_BUCKET: R2Bucket;
  JWT_SECRET: string;
  WHATSAPP_PHONE_NUMBER_ID: string;
  WHATSAPP_TOKEN: string;
};

const user = new Hono<{
  Bindings: Bindings;
  Variables: { jwtPayload: JwtPayload };
}>();

// All routes under /user require permanent JWT auth
user.use("*", requireAuth);

/** GET /user/me — authenticated user's full profile. */
user.get("/me", async (c) => {
  const { jwtPayload } = c.var;
  const db = userQueries(c.env.DB);

  const currentUser = await db.findById(jwtPayload.sub!);
  if (!currentUser) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  return c.json({ user: currentUser });
});

/** PUT /user/me — update name / avatar. */
user.put("/me", async (c) => {
  const { jwtPayload } = c.var;
  const db = userQueries(c.env.DB);
  const contentType = c.req.header("Content-Type") ?? "";

  const currentUser = await db.findById(jwtPayload.sub!);
  if (!currentUser) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  let nameField: string | undefined;
  let avatarFile: File | undefined;

  if (contentType.includes("multipart/form-data")) {
    const formData = await c.req.parseBody();
    nameField = formData["name"] as string | undefined;
    avatarFile = formData["avatar"] as File | undefined;
  } else {
    const body = await c.req.json<{ name?: string }>();
    nameField = body.name;
  }

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

/** POST /user/invite — invite a user by phone + name (optional avatar). */
user.post("/invite", async (c) => {
  const db = userQueries(c.env.DB);
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
    const body = await c.req.json<InviteRequest>();
    phone = body.phone;
    name = body.name;
  }

  if (!phone || !phone.startsWith("+")) {
    return c.json(
      { error: "validation", message: "Phone must be in E.164 format" },
      400,
    );
  }
  if (!name || name.trim().length === 0) {
    return c.json({ error: "validation", message: "Name is required" }, 400);
  }

  const existing = await db.findByPhone(phone);
  if (existing) {
    return c.json({ user: existing, invited: false });
  }

  // Create invited user
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

  const invitedUser = {
    id: userId,
    phone,
    name: name.trim(),
    level: Level.Normal,
    status: Status.Invited,
    created: nowEpoch,
    avatar_url: avatarUrl,
    created_at: now,
    updated_at: now,
  };

  await db.create(invitedUser);

  return c.json({ user: invitedUser, invited: true }, 201);
});

/** PUT /user/:id/level — promote / demote (Super only). */
user.put("/:id/level", requireSuper, async (c) => {
  const targetId = c.req.param("id");
  const db = userQueries(c.env.DB);
  const body = await c.req.json<UpdateLevelRequest>();

  if (
    body.level !== Level.Normal &&
    body.level !== Level.System &&
    body.level !== Level.Super
  ) {
    return c.json({ error: "validation", message: "Invalid level" }, 400);
  }

  const updated = await db.updateLevel(targetId, body.level);
  if (!updated) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  return c.json({ user: updated });
});

/** PUT /user/:id/status — block / unblock / delete (Super only). */
user.put("/:id/status", requireSuper, async (c) => {
  const targetId = c.req.param("id");
  const db = userQueries(c.env.DB);
  const body = await c.req.json<UpdateStatusRequest>();

  if (
    body.status !== Status.Invited &&
    body.status !== Status.Active &&
    body.status !== Status.Suspended &&
    body.status !== Status.Deleted
  ) {
    return c.json({ error: "validation", message: "Invalid status" }, 400);
  }

  const updated = await db.updateStatus(targetId, body.status);
  if (!updated) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  return c.json({ user: updated });
});

/** POST /user/change-phone/request — send verification to new phone. */
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

/** POST /user/change-phone/verify — verify and apply phone change. */
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

  const token = await createPermanentToken(
    c.env,
    updated.id,
    updated.phone,
    updated.level,
  );
  return c.json({ token, user: updated });
});

export default user;
