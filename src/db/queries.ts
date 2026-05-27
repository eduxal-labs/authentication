import type { User } from "../types";
import { Level, Status } from "../types";

export function userQueries(db: D1Database) {
  return {
    async findById(id: string): Promise<User | null> {
      const result = await db
        .prepare("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .first<User>();
      return result ?? null;
    },

    async findByPhone(phone: string): Promise<User | null> {
      const result = await db
        .prepare("SELECT * FROM users WHERE phone = ?")
        .bind(phone)
        .first<User>();
      return result ?? null;
    },

    async create(user: User): Promise<void> {
      await db
        .prepare(
          "INSERT INTO users (id, phone, name, level, status, created, avatar_url, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(
          user.id,
          user.phone,
          user.name,
          user.level,
          user.status,
          user.created,
          user.avatar_url,
          user.created_at,
          user.updated_at,
        )
        .run();
    },

    /** Update name + avatar. */
    async updateProfile(
      id: string,
      fields: { name?: string; avatar_url?: string },
    ): Promise<User | null> {
      const now = new Date().toISOString();
      const sets: string[] = ["updated_at = ?"];
      const values: unknown[] = [now];

      if (fields.name !== undefined) {
        sets.push("name = ?");
        values.push(fields.name);
      }
      if (fields.avatar_url !== undefined) {
        sets.push("avatar_url = ?");
        values.push(fields.avatar_url);
      }

      values.push(id);
      await db
        .prepare(`UPDATE users SET ${sets.join(", ")} WHERE id = ?`)
        .bind(...values)
        .run();

      return this.findById(id);
    },

    async updatePhone(id: string, newPhone: string): Promise<User | null> {
      const now = new Date().toISOString();
      await db
        .prepare("UPDATE users SET phone = ?, updated_at = ? WHERE id = ?")
        .bind(newPhone, now, id)
        .run();
      return this.findById(id);
    },

    /** Promote or demote a user's level. */
    async updateLevel(id: string, level: number): Promise<User | null> {
      await db
        .prepare("UPDATE users SET level = ?, updated_at = ? WHERE id = ?")
        .bind(level, new Date().toISOString(), id)
        .run();
      return this.findById(id);
    },

    /** Change a user's status (block / unblock / delete). */
    async updateStatus(id: string, status: number): Promise<User | null> {
      await db
        .prepare("UPDATE users SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status, new Date().toISOString(), id)
        .run();
      return this.findById(id);
    },

    /** Transition status on login: Deleted/Invited → Active. */
    async activateIfNeeded(user: User): Promise<User> {
      if (user.status === Status.Deleted || user.status === Status.Invited) {
        await db
          .prepare("UPDATE users SET status = ?, updated_at = ? WHERE id = ?")
          .bind(Status.Active, new Date().toISOString(), user.id)
          .run();
        return { ...user, status: Status.Active as typeof user.status };
      }
      return user;
    },

    /** Public profile: no phone, no level, no status. */
    async getPublicProfile(
      id: string,
    ): Promise<Pick<User, "id" | "name" | "avatar_url" | "created_at"> | null> {
      const result = await db
        .prepare(
          "SELECT id, name, avatar_url, created_at FROM users WHERE id = ?",
        )
        .bind(id)
        .first<Pick<User, "id" | "name" | "avatar_url" | "created_at">>();
      return result ?? null;
    },

    /** FTS5 search: name OR phone prefix. */
    async search(
      query: string,
      limit: number = 20,
      offset: number = 0,
    ): Promise<{ users: User[]; total: number }> {
      const escaped = query.replace(/"/g, '""');
      const countResult = await db
        .prepare(
          "SELECT COUNT(*) as total FROM users_fts WHERE users_fts MATCH ?",
        )
        .bind(`"${escaped}"`)
        .first<{ total: number }>();

      const rows = await db
        .prepare(
          "SELECT u.* FROM users u INNER JOIN users_fts f ON u.rowid = f.rowid WHERE f MATCH ? ORDER BY rank LIMIT ? OFFSET ?",
        )
        .bind(`"${escaped}"`, limit, offset)
        .all<User>();

      return {
        users: rows.results,
        total: countResult?.total ?? 0,
      };
    },

    /** List users (paginated). */
    async list(
      limit: number = 20,
      offset: number = 0,
    ): Promise<{ users: User[]; total: number }> {
      const countResult = await db
        .prepare("SELECT COUNT(*) as total FROM users")
        .first<{ total: number }>();

      const rows = await db
        .prepare("SELECT * FROM users ORDER BY created DESC LIMIT ? OFFSET ?")
        .bind(limit, offset)
        .all<User>();

      return {
        users: rows.results,
        total: countResult?.total ?? 0,
      };
    },

    /** Edit any user field (admin). */
    async adminEdit(
      id: string,
      fields: {
        name?: string;
        phone?: string;
        level?: number;
        status?: number;
      },
    ): Promise<User | null> {
      const now = new Date().toISOString();
      const sets: string[] = ["updated_at = ?"];
      const values: unknown[] = [now];

      if (fields.name !== undefined) {
        sets.push("name = ?");
        values.push(fields.name);
      }
      if (fields.phone !== undefined) {
        sets.push("phone = ?");
        values.push(fields.phone);
      }
      if (fields.level !== undefined) {
        sets.push("level = ?");
        values.push(fields.level);
      }
      if (fields.status !== undefined) {
        sets.push("status = ?");
        values.push(fields.status);
      }

      values.push(id);
      await db
        .prepare(`UPDATE users SET ${sets.join(", ")} WHERE id = ?`)
        .bind(...values)
        .run();

      return this.findById(id);
    },

    /** Hard-delete user + their avatar key from R2 (caller handles R2). */
    async purge(id: string): Promise<{ avatarKey: string | null }> {
      const user = await this.findById(id);
      const avatarKey = user?.avatar_url ?? null;
      await db.prepare("DELETE FROM users WHERE id = ?").bind(id).run();
      return { avatarKey };
    },
  };
}

export type UserQueries = ReturnType<typeof userQueries>;
