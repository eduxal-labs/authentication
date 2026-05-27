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

-- FTS5 search index for name + phone
CREATE VIRTUAL TABLE IF NOT EXISTS users_fts USING fts5(name, phone, content='');

-- Keep FTS5 in sync on insert
CREATE TRIGGER IF NOT EXISTS users_fts_insert AFTER INSERT ON users BEGIN
  INSERT INTO users_fts(rowid, name, phone) VALUES (new.rowid, new.name, new.phone);
END;

-- Keep FTS5 in sync on update
CREATE TRIGGER IF NOT EXISTS users_fts_update AFTER UPDATE ON users BEGIN
  UPDATE users_fts SET name = new.name, phone = new.phone WHERE rowid = old.rowid;
END;

-- Keep FTS5 in sync on delete
CREATE TRIGGER IF NOT EXISTS users_fts_delete AFTER DELETE ON users BEGIN
  INSERT INTO users_fts(users_fts, rowid, name, phone) VALUES('delete', old.rowid, old.name, old.phone);
END;
