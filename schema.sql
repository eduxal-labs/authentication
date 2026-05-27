CREATE TABLE IF NOT EXISTS users (
  id         TEXT PRIMARY KEY,
  phone      TEXT NOT NULL UNIQUE,
  name       TEXT NOT NULL,
  avatar_url TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- Migration: add avatar_url if missing from an earlier schema
ALTER TABLE users ADD COLUMN avatar_url TEXT;

CREATE INDEX IF NOT EXISTS idx_users_phone ON users(phone);
