import { Hono } from "hono";
import { requireSuper } from "../middleware/auth";
import { userQueries } from "../db/queries";
import type { AdminEditUserRequest, JwtPayload } from "../types";

type Bindings = {
  DB: D1Database;
  AVATARS_BUCKET: R2Bucket;
  JWT_SECRET: string;
};

const admin = new Hono<{
  Bindings: Bindings;
  Variables: { jwtPayload: JwtPayload };
}>();

// All admin routes require Super access
admin.use("*", requireSuper);

/** GET /admin/users — list users (paginated). */
admin.get("/users", async (c) => {
  const limit = Math.min(parseInt(c.req.query("limit") ?? "20") || 20, 100);
  const offset = parseInt(c.req.query("offset") ?? "0") || 0;
  const db = userQueries(c.env.DB);

  const { users, total } = await db.list(limit, offset);
  return c.json({ users, total, limit, offset });
});

/** GET /admin/users/search?q= — FTS5 search by name or phone. */
admin.get("/users/search", async (c) => {
  const q = c.req.query("q");
  if (!q || q.trim().length === 0) {
    return c.json({ error: "validation", message: "Query 'q' is required" }, 400);
  }

  const limit = Math.min(parseInt(c.req.query("limit") ?? "20") || 20, 100);
  const offset = parseInt(c.req.query("offset") ?? "0") || 0;
  const db = userQueries(c.env.DB);

  const { users, total } = await db.search(q.trim(), limit, offset);
  return c.json({ users, total, limit, offset, query: q });
});

/** PUT /admin/users/:id — edit any user field. */
admin.put("/users/:id", async (c) => {
  const targetId = c.req.param("id");
  const db = userQueries(c.env.DB);
  const body = await c.req.json<AdminEditUserRequest>();

  const existing = await db.findById(targetId);
  if (!existing) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  const updates: Parameters<typeof db.adminEdit>[1] = {};
  if (body.name !== undefined) updates.name = body.name;
  if (body.phone !== undefined) {
    // Check phone not taken by another user
    const byPhone = await db.findByPhone(body.phone);
    if (byPhone && byPhone.id !== targetId) {
      return c.json({ error: "conflict", message: "Phone taken" }, 409);
    }
    updates.phone = body.phone;
  }
  if (body.level !== undefined) updates.level = body.level;
  if (body.status !== undefined) updates.status = body.status;

  if (Object.keys(updates).length === 0) {
    return c.json({ error: "validation", message: "No fields to update" }, 400);
  }

  const updated = await db.adminEdit(targetId, updates);
  return c.json({ user: updated });
});

/** DELETE /admin/users/:id — soft-delete (status=Deleted) + remove avatar. */
admin.delete("/users/:id", async (c) => {
  const targetId = c.req.param("id");
  const db = userQueries(c.env.DB);

  const existing = await db.findById(targetId);
  if (!existing) {
    return c.json({ error: "not_found", message: "User not found" }, 404);
  }

  // Delete avatar from R2
  if (existing.avatar_url) {
    try {
      await c.env.AVATARS_BUCKET.delete(existing.avatar_url);
    } catch {
      // ignore
    }
  }

  await db.adminEdit(targetId, { status: 3 });
  await db.updateProfile(targetId, { avatar_url: "" });

  return c.json({ success: true, message: "User soft-deleted" });
});

/** DELETE /admin/users/:id/purge — hard-delete from DB + R2. */
admin.delete("/users/:id/purge", async (c) => {
  const targetId = c.req.param("id");
  const db = userQueries(c.env.DB);

  const { avatarKey } = await db.purge(targetId);

  // Delete avatar from R2 if it existed
  if (avatarKey) {
    try {
      await c.env.AVATARS_BUCKET.delete(avatarKey);
    } catch {
      // ignore
    }
  }

  return c.json({ success: true, message: "User purged" });
});

export default admin;
