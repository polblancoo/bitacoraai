CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    token_version INTEGER NOT NULL DEFAULT 0,
    email TEXT,
    email_verified INTEGER NOT NULL DEFAULT 0,
    email_code_hash TEXT,
    email_code_expires TEXT,
    telegram_chat_id INTEGER,
    tg_code_hash TEXT,
    tg_code_expires TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS teams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    owner_id INTEGER NOT NULL REFERENCES users(id)
);
CREATE TABLE IF NOT EXISTS team_members (
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'member',
    PRIMARY KEY (team_id, user_id)
);
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    color TEXT NOT NULL DEFAULT '#7aa2f7'
);
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'todo',
    priority INTEGER NOT NULL DEFAULT 1,
    position REAL NOT NULL DEFAULT 0,
    assignee_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    due TEXT,
    recurrence TEXT,
    remind_state INTEGER NOT NULL DEFAULT 0,
    parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
    start TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_comments_task ON comments(task_id);
CREATE TABLE IF NOT EXISTS labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL COLLATE NOCASE,
    color TEXT NOT NULL DEFAULT '#bb9af7',
    UNIQUE (team_id, name)
);
CREATE TABLE IF NOT EXISTS task_labels (
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, label_id)
);
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
CREATE TABLE IF NOT EXISTS invites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL,
    max_uses INTEGER NOT NULL,
    uses INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS task_deps (
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    depends_on INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, depends_on)
);
CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    team_id INTEGER,
    task_id INTEGER,
    text TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notif_user ON notifications(user_id, read, id);
CREATE TABLE IF NOT EXISTS canvas_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    w REAL NOT NULL,
    h REAL NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    color TEXT NOT NULL DEFAULT '#e0af68',
    task_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS canvas_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    from_id INTEGER NOT NULL REFERENCES canvas_items(id) ON DELETE CASCADE,
    to_id INTEGER NOT NULL REFERENCES canvas_items(id) ON DELETE CASCADE,
    UNIQUE (from_id, to_id)
);
CREATE INDEX IF NOT EXISTS idx_canvas_items_project ON canvas_items(project_id);

CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    size INTEGER NOT NULL,
    data BLOB NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attachments_task ON attachments(task_id);
CREATE TABLE IF NOT EXISTS activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),
    text TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_activity_project ON activity(project_id, id);

CREATE TABLE IF NOT EXISTS google_links (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    refresh_token TEXT NOT NULL,
    linked_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS google_events (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    task_id INTEGER NOT NULL,
    event_id TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    PRIMARY KEY (user_id, task_id)
);

CREATE TABLE IF NOT EXISTS api_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    hash TEXT NOT NULL UNIQUE,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    last_used TEXT
);

CREATE TABLE IF NOT EXISTS task_grants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ssh_targets (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    user TEXT NOT NULL,
    key_path TEXT NOT NULL,
    host_fp TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS task_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    bot_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    model TEXT NOT NULL DEFAULT '',
    started_at TEXT NOT NULL,
    seconds INTEGER NOT NULL DEFAULT 0,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cache_create INTEGER NOT NULL DEFAULT 0,
    cache_read INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0,
    ok INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_task_runs_project ON task_runs(project_id, started_at);
CREATE INDEX IF NOT EXISTS idx_task_runs_task ON task_runs(task_id);

CREATE TABLE IF NOT EXISTS schedules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    priority INTEGER NOT NULL DEFAULT 1,
    assignee_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    freq TEXT NOT NULL,
    weekday INTEGER NOT NULL DEFAULT 0,
    monthday INTEGER NOT NULL DEFAULT 1,
    at_minute INTEGER NOT NULL DEFAULT 540,
    tz_offset INTEGER NOT NULL DEFAULT 0,
    skip_if_open INTEGER NOT NULL DEFAULT 1,
    active INTEGER NOT NULL DEFAULT 1,
    next_run TEXT NOT NULL,
    last_run TEXT,
    last_task_id INTEGER,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS schedules_due ON schedules (active, next_run);

CREATE TABLE IF NOT EXISTS bot_runners (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 0,
    set_by INTEGER
);

CREATE TABLE IF NOT EXISTS github_links (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    token TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT '',
    linked_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS report_subs (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    freq TEXT NOT NULL,
    weekday INTEGER NOT NULL DEFAULT 0,
    at_minute INTEGER NOT NULL DEFAULT 540,
    tz_offset INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    via_telegram INTEGER NOT NULL DEFAULT 1,
    via_email INTEGER NOT NULL DEFAULT 1,
    next_run TEXT NOT NULL,
    last_run TEXT,
    PRIMARY KEY (user_id, team_id)
);
CREATE INDEX IF NOT EXISTS report_subs_due ON report_subs (active, next_run);

-- Revisión humana del trabajo de las IAs (una fila por pedido de revisión; la última de cada tarea es la vigente)
CREATE TABLE IF NOT EXISTS task_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    requested_by INTEGER NOT NULL,
    reviewer_id INTEGER,
    state TEXT NOT NULL DEFAULT 'pending',
    note TEXT NOT NULL DEFAULT '',
    requested_at TEXT NOT NULL,
    decided_by INTEGER,
    decided_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_task_reviews_task ON task_reviews(task_id, id);

-- Cómo recibe cada persona los avisos por Telegram y email: uno por uno, o un resumen a una hora fija (por defecto, diario a las 9)
CREATE TABLE IF NOT EXISTS notif_prefs (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    mode TEXT NOT NULL DEFAULT 'digest',
    hour INTEGER NOT NULL DEFAULT 9,
    tz_offset INTEGER,
    last_digest TEXT
);

-- Perfil de uso del equipo (dev, content, factory, other): define la interfaz por defecto
CREATE TABLE IF NOT EXISTS team_profile (
    team_id INTEGER PRIMARY KEY REFERENCES teams(id) ON DELETE CASCADE,
    profile TEXT NOT NULL
);

-- Proyecto ↔ repositorio de GitHub, y tarea ↔ issue de donde se importó
CREATE TABLE IF NOT EXISTS project_github (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    repo TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS task_github (
    task_id INTEGER PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    url TEXT NOT NULL
);

-- Ajustes de GitHub por proyecto (cada casilla se prende o se apaga sola) y estado de la última sincronización
CREATE TABLE IF NOT EXISTS project_github_cfg (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    owner_user INTEGER NOT NULL,
    auto_sync INTEGER NOT NULL DEFAULT 0,
    sync_comments INTEGER NOT NULL DEFAULT 0,
    publish_issues INTEGER NOT NULL DEFAULT 0,
    publish_since TEXT NOT NULL DEFAULT '',
    ai_pr INTEGER NOT NULL DEFAULT 0,
    ci_status INTEGER NOT NULL DEFAULT 0,
    activity INTEGER NOT NULL DEFAULT 0,
    webhook INTEGER NOT NULL DEFAULT 0,
    webhook_secret TEXT NOT NULL DEFAULT '',
    last_sha TEXT NOT NULL DEFAULT '',
    ci_state TEXT NOT NULL DEFAULT '',
    last_sync TEXT NOT NULL DEFAULT '',
    last_msg TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    project_id INTEGER,
    from_user INTEGER NOT NULL REFERENCES users(id),
    to_user INTEGER,
    body TEXT NOT NULL,
    file_id INTEGER,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_team ON chat_messages(team_id, id);
CREATE TABLE IF NOT EXISTS chat_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    uploader INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    size INTEGER NOT NULL,
    data BLOB NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS chat_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    team_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    detail TEXT NOT NULL,
    ip TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_audit_team ON chat_audit(team_id, id);
