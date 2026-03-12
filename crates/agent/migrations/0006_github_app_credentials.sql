CREATE TABLE IF NOT EXISTS github_app_credentials (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL,
    app_slug TEXT,
    client_id TEXT NOT NULL,
    client_secret_encrypted TEXT NOT NULL,
    webhook_secret_encrypted TEXT NOT NULL,
    private_key_pem_encrypted TEXT NOT NULL,
    app_name TEXT NOT NULL,
    app_html_url TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_github_app_credentials_app_id
ON github_app_credentials(app_id);