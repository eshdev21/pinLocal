pub const CREATE_BOARDS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS boards (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL,
  path        TEXT NOT NULL UNIQUE COLLATE NOCASE,
  cover_image TEXT,
  image_count INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  is_missing  INTEGER NOT NULL DEFAULT 0,
  needs_ai_sync INTEGER NOT NULL DEFAULT 0
);";

pub const CREATE_IMAGES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS images (
  id               INTEGER PRIMARY KEY,
  filename         TEXT NOT NULL,
  path             TEXT NOT NULL UNIQUE COLLATE NOCASE,
  board_id         INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
  thumb_path       TEXT,
  width            INTEGER,
  height           INTEGER,
  size_bytes       INTEGER NOT NULL DEFAULT 0,
  mtime            INTEGER NOT NULL DEFAULT 0,
  created_at       INTEGER NOT NULL,
  thumbnail_status TEXT NOT NULL DEFAULT 'pending',
  is_missing       INTEGER NOT NULL DEFAULT 0
);";

pub const CREATE_WORKSPACES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS workspaces (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);";

pub const CREATE_WORKSPACE_FOLDERS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS workspace_folders (
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  board_id     INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
  PRIMARY KEY (workspace_id, board_id)
);";

pub const CREATE_APP_STATE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS app_state (
  key   TEXT PRIMARY KEY,
  value TEXT
);";

pub const CREATE_BACKGROUND_TASKS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS background_tasks (
  id             TEXT PRIMARY KEY,
  task_type      TEXT NOT NULL,
  status         TEXT NOT NULL, -- 'pending', 'running', 'completed', 'failed', 'interrupted'
  message        TEXT,
  progress_done  INTEGER DEFAULT 0,
  progress_total INTEGER DEFAULT 0,
  updated_at     INTEGER NOT NULL
);";

pub const CREATE_INDEXES: &str = "
CREATE INDEX IF NOT EXISTS idx_images_mtime        ON images(mtime);
CREATE INDEX IF NOT EXISTS idx_images_thumb_status ON images(thumbnail_status);
CREATE INDEX IF NOT EXISTS idx_images_missing      ON images(is_missing);
CREATE INDEX IF NOT EXISTS idx_tasks_status        ON background_tasks(status);
CREATE INDEX IF NOT EXISTS idx_images_board_mtime  ON images(board_id, mtime DESC);
";

pub const CREATE_TRIGGERS: &str = "";
