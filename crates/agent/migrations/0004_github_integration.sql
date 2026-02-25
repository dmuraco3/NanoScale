CREATE TABLE IF NOT EXISTS github_user_links (
    id TEXT PRIMARY KEY,
    local_user_id TEXT NOT NULL,
    github_user_id INTEGER NOT NULL,
    github_login TEXT NOT NULL,
    access_token_encrypted TEXT NOT NULL,
    refresh_token_encrypted TEXT,
    token_expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(local_user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_github_user_links_local_user_id
ON github_user_links(local_user_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_github_user_links_github_user_id
ON github_user_links(github_user_id);

CREATE TABLE IF NOT EXISTS github_installations (
    id TEXT PRIMARY KEY,
    local_user_id TEXT NOT NULL,
    installation_id INTEGER NOT NULL,
    account_login TEXT NOT NULL,
    account_type TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(local_user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_github_installations_installation_id
ON github_installations(installation_id);

CREATE INDEX IF NOT EXISTS idx_github_installations_local_user_id
ON github_installations(local_user_id);

CREATE TABLE IF NOT EXISTS github_repositories (
    id TEXT PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    repo_id INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    owner_login TEXT NOT NULL,
    name TEXT NOT NULL,
    full_name TEXT NOT NULL,
    default_branch TEXT NOT NULL,
    is_private BOOLEAN NOT NULL,
    html_url TEXT NOT NULL,
    clone_url TEXT NOT NULL,
    archived BOOLEAN NOT NULL DEFAULT 0,
    disabled BOOLEAN NOT NULL DEFAULT 0,
    last_synced_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_github_repositories_repo_id
ON github_repositories(repo_id);

CREATE INDEX IF NOT EXISTS idx_github_repositories_installation_id
ON github_repositories(installation_id);

CREATE INDEX IF NOT EXISTS idx_github_repositories_full_name
ON github_repositories(full_name);

CREATE TABLE IF NOT EXISTS project_github_links (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    installation_id INTEGER NOT NULL,
    repo_id INTEGER NOT NULL,
    repo_node_id TEXT NOT NULL,
    owner_login TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    full_name TEXT NOT NULL,
    default_branch TEXT NOT NULL,
    selected_branch TEXT NOT NULL,
    webhook_id INTEGER,
    webhook_secret_encrypted TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(project_id) REFERENCES projects(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_project_github_links_project_id
ON project_github_links(project_id);

CREATE INDEX IF NOT EXISTS idx_project_github_links_repo_branch
ON project_github_links(repo_id, selected_branch);

CREATE TABLE IF NOT EXISTS github_webhook_deliveries (
    id TEXT PRIMARY KEY,
    delivery_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    repo_id INTEGER,
    ref TEXT,
    head_commit TEXT,
    handled BOOLEAN NOT NULL DEFAULT 0,
    status_code INTEGER,
    error_message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_github_webhook_deliveries_delivery_id
ON github_webhook_deliveries(delivery_id);

ALTER TABLE projects
ADD COLUMN source_provider TEXT NOT NULL DEFAULT 'manual';

ALTER TABLE projects
ADD COLUMN source_repo_id INTEGER;
