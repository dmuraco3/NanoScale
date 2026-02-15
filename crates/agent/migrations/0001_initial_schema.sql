CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    status TEXT NOT NULL,
    secret_key TEXT NOT NULL,
    public_key TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL UNIQUE,
    repo_url TEXT NOT NULL,
    branch TEXT DEFAULT 'main',
    node_version TEXT DEFAULT '20',
    install_command TEXT DEFAULT 'bun install --frozen-lockfile',
    build_command TEXT DEFAULT 'bun run build',
    start_command TEXT DEFAULT 'bun run start',
    env_vars TEXT NOT NULL,
    port INTEGER NOT NULL,
    domain TEXT,
    scale_to_zero BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(server_id) REFERENCES servers(id)
);

CREATE INDEX IF NOT EXISTS idx_projects_server_id ON projects(server_id);
