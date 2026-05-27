CREATE TABLE IF NOT EXISTS users (
  id         TEXT PRIMARY KEY,
  phone      TEXT NOT NULL UNIQUE,
  name       TEXT NOT NULL,
  level      INTEGER NOT NULL DEFAULT 0,
  status     INTEGER NOT NULL DEFAULT 0,
  created    INTEGER NOT NULL,
  avatar_url TEXT,
  created_at TEXT,
  updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_users_phone ON users(phone);
