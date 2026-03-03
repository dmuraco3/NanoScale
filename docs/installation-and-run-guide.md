# NanoScale Installation & Run Guide

This guide covers the current MVP flow in this repository:

- Install baseline host prerequisites with `scripts/install.sh`
- Run the Rust agent as an Orchestrator via `systemd`
- Run the Next.js dashboard
- Add Worker nodes with a join token
- Apply OTA updates from the Settings page

## 1) Prerequisites

### Host OS

- Linux host (Ubuntu/Debian recommended)
- `sudo` access (installer must run as root)

### Toolchains (for building from source)

- Rust `1.93.0` (pinned by `rust-toolchain.toml`)
- Bun `1.2.20` (pinned by `.bun-version` and root `packageManager`)

If you use `rustup` and Bun version managers, align to those pinned versions.

## 2) Clone the Repository

```bash
git clone <your-repo-url> nanoscale
cd nanoscale
```

If you are installing from a packaged GitHub Release instead of source, skip this section and use **Section 3.1**.

## 3) Install Baseline System Dependencies

Run the installer as root on each machine.

### 3.1 Package Install (Default)

Use this path when installing from a release archive. This is the default installer behavior.

1) Download and extract release:

```bash
curl -fL -o nanoscale-release.tar.gz https://github.com/<owner>/<repo>/releases/latest/download/nanoscale-release.tar.gz
mkdir nanoscale-release
tar -xzf nanoscale-release.tar.gz -C nanoscale-release
cd nanoscale-release
```

2) Run installer (package mode is default):

```bash
sudo ./install.sh
```

3) Validate:

```bash
curl -i http://127.0.0.1:4000/api/health
sudo systemctl status nanoscale.service
sudo systemctl status nanoscale-dashboard.service
```

Optional env override:

- `NANOSCALE_REPO_SLUG=<owner/repo>` to install from a different GitHub repo.

What this install command does:

- creates `nanoscale` user/group and required directories
- writes default `/opt/nanoscale/config.json` if missing
- installs/enables/starts:
  - `nanoscale.service` (backend)
  - `nanoscale-dashboard.service` (Next standalone dashboard)
- installs `/usr/local/bin/nanoscale-system-updater` and locked-down sudoers policy
- configures firewall rules

### 3.2 Source Install (Optional)

If you cloned the repository and want to build binaries locally instead of using packaged artifacts:

```bash
sudo ./scripts/install.sh --source
```

### Orchestrator host

```bash
sudo ./scripts/install.sh --role orchestrator
```

### Worker host

Use this after you generate a join token (see Section 6).

```bash
sudo ./scripts/install.sh --join <JOIN_TOKEN>
```

What the installer currently does:

- Ensures `curl`, `git`, `nginx`, `sqlite3`, `ufw`
- Creates `nanoscale` user/group
- Creates `/opt/nanoscale/{bin,data,sites,config,logs,tmp}`
- Installs `/etc/systemd/system/nanoscale.service`
- Installs root-owned wrapper `/usr/local/bin/nanoscale-system-updater` (`0700`)
- Installs `/etc/sudoers.d/nanoscale` allowing only:
  - `/bin/systemctl restart nanoscale.service`
  - `/usr/local/bin/nanoscale-system-updater`
- Enables firewall rules for ports `22`, `80`, `443`, `4000`

## 4) Build the Agent Binary

If you used Section 3.1 package installation, you can skip this section.

From repo root:

```bash
cargo build --release -p agent
```

Binary path:

- `target/release/agent`

Installer-managed runtime binary path:

- `/opt/nanoscale/backend-bin`

## 5) Run Orchestrator (Agent + API)

On the orchestrator machine:

1) Edit backend config values in `/opt/nanoscale/config.json` (created by installer):

```json
{
  "database_path": "/opt/nanoscale/data/nanoscale.db",
  "tls_email": "admin@mydomain.com",
  "orchestrator": {
    "bind_address": "0.0.0.0:4000",
    "server_id": "orchestrator-local",
    "server_name": "orchestrator",
    "worker_ip": "127.0.0.1",
    "base_domain": "mydomain.com"
  },
  "worker": {
    "orchestrator_url": "http://127.0.0.1:4000",
    "ip": "127.0.0.1",
    "name": "worker-node",
    "bind": "0.0.0.0:4000"
  }
}
```

`orchestrator.base_domain` is optional. When set, newly created projects get a `domain` like:

- `test-app.mydomain.com`

`tls_email` is optional, but required if you want NanoScale to automatically request and install
Let's Encrypt certificates for project domains. You can also set it via the environment variable
`NANOSCALE_TLS_EMAIL`.

2) Start orchestrator:

```bash
sudo systemctl start nanoscale.service
sudo systemctl status nanoscale.service
```

Note: assigning a domain does not automatically configure DNS. For the URL to resolve publicly you must:

- Own/control the base domain
- Point the project subdomain(s) to the public IP of the machine running the project (worker), or to a load balancer that routes by `Host`
- (Optional) Set up TLS/HTTPS separately (Let’s Encrypt, cert manager, etc.)

On startup you should see:

- DB initialized message
- listening address (default port `4000`)

## 6) Run Dashboard (Control Plane UI)

In a second terminal on the orchestrator machine, from repo root:

```bash
bun install
bun run dev
```

Default dashboard URL:

- `http://localhost:3000`

First-run flow:

1. Open `/setup` to create the initial admin user
2. Login through `/login`
3. Use Servers page to generate a cluster join token

## 7) Add a Worker Node

On a worker machine:

1. Clone repo and run installer:

```bash
sudo ./scripts/install.sh --join <JOIN_TOKEN>
```

2. Build agent:

```bash
cargo build --release -p agent
```

3. Start worker and join orchestrator:

```bash
./target/release/agent --join <JOIN_TOKEN>
```

Worker connection/runtime values are loaded from `/opt/nanoscale/config.json` under the `worker` section.

If successful, the worker reports it joined the cluster and starts internal API endpoints on port `4000`.

## 8) Quick Validation Checklist

- Orchestrator terminal shows API listening on `0.0.0.0:4000`
- Dashboard loads at `http://localhost:3000`
- Setup + login succeeds
- Generated join token allows worker to join
- Worker appears in Servers view

## 9) Notes for Current MVP State

- The orchestrator agent is installed and managed through `nanoscale.service`.
- The dashboard may still be run with `bun run dev` for development.
- For production dashboard runtime, use a process manager or service unit in your host environment.
- Keep Rust/Bun versions pinned to avoid CI/local mismatch.

## 10) Release Packaging and OTA Updates

NanoScale release packaging is built by GitHub Actions on published releases.

- Workflow: `.github/workflows/release-package.yml`
- Release assets:
  - `nanoscale-release.tar.gz`
  - `manifest.json`

The release tarball contains:

- `/backend-bin`
- `/server.js`
- `/package.json`
- `/.next/`
- `/public/`
- `/.next/static/`

`manifest.json` contains:

- `version`: release tag/version
- `requires_system_update`: boolean

System-update signaling:

- To force a system-level update path, include this in the GitHub Release notes/body:

```text
requires_system_update: true
```

When a user clicks **Update** in Settings:

1. Dashboard calls `POST /api/admin/update` and receives `202 Accepted`.
2. Agent background task fetches latest release manifest.
3. If `requires_system_update=true`, agent runs:
   - `sudo /usr/local/bin/nanoscale-system-updater`
4. If `requires_system_update=false`, agent downloads and installs code release:
   - extract to `/opt/nanoscale-staging`
   - atomic swap with `/opt/nanoscale`
   - `sudo /bin/systemctl restart nanoscale.service`
5. Dashboard polls `GET /api/health` every 2s and reloads once healthy.

## 11) GitHub Integration (Self-Hosted Setup)

Use this section if you want dashboard users to connect GitHub accounts, select repositories (including private repos), and auto-redeploy on push.

### 11.1 Prerequisites

- A public HTTPS URL that reaches your orchestrator API (for OAuth callback + webhook delivery).
- Access to create a GitHub App in your GitHub organization or personal account.
- A place to store the GitHub App private key PEM on the orchestrator host.

### 11.2 Create the GitHub App (GitHub Dashboard)

In GitHub, go to **Settings → Developer settings → GitHub Apps → New GitHub App** and set:

- **GitHub App name**: your choice (example: `NanoScale Self-Hosted`)
- **Homepage URL**: `https://<your-public-domain>`
- **Callback URL**:

```text
https://<your-public-domain>/api/integrations/github/callback
```

- **Webhook URL**:

```text
https://<your-public-domain>/api/integrations/github/webhook
```

- **Webhook secret**: generate a strong random secret and save it for NanoScale config.

Generate webhook secret example:

```bash
openssl rand -hex 32
```

If `openssl` is unavailable:

```bash
python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
```

Repository permissions (minimum):

- **Metadata**: Read-only
- **Contents**: Read-only
- **Webhooks**: Read and write

Subscribe to webhook events:

- **Push**

After creating the app:

1. Generate a private key (`.pem`) and copy it to your orchestrator host, for example:
   - `/opt/nanoscale/config/github-app.pem`
2. Record:
   - App ID
   - App slug
   - Client ID
   - Client secret

Where to find each value in GitHub App settings:

- App ID: GitHub App page → **About** section
- App slug: GitHub App page URL and **About** section (usually lowercase-hyphenated app name)
- Client ID: GitHub App page → **About** section
- Client secret: GitHub App page → **Private keys** (generate if missing)

Recommended host-side key setup:

```bash
sudo install -m 600 -o nanoscale -g nanoscale github-app.pem /opt/nanoscale/config/github-app.pem
sudo ls -l /opt/nanoscale/config/github-app.pem
```

Expected permissions should be owner-read/write only (`-rw-------`) and owned by `nanoscale:nanoscale`.

### 11.3 Configure NanoScale (`config.json` or env vars)

GitHub settings are loaded from `/opt/nanoscale/config.json` under `github`, and each value can also be provided via env vars.

Example `config.json` snippet:

```json
{
  "github": {
    "enabled": true,
    "app_id": "<github_app_id>",
    "app_slug": "<github_app_slug>",
    "client_id": "<github_client_id>",
    "client_secret": "<github_client_secret>",
    "private_key_path": "/opt/nanoscale/config/github-app.pem",
    "webhook_secret": "<strong-random-secret>",
    "public_base_url": "https://<your-public-domain>",
    "encryption_key": "<base64-encoded-32-byte-key>"
  }
}
```

Equivalent env vars (optional alternative):

```bash
NANOSCALE_GITHUB_ENABLED=true
NANOSCALE_GITHUB_APP_ID=<github_app_id>
NANOSCALE_GITHUB_APP_SLUG=<github_app_slug>
NANOSCALE_GITHUB_APP_CLIENT_ID=<github_client_id>
NANOSCALE_GITHUB_APP_CLIENT_SECRET=<github_client_secret>
NANOSCALE_GITHUB_APP_PRIVATE_KEY_PATH=/opt/nanoscale/config/github-app.pem
NANOSCALE_GITHUB_WEBHOOK_SECRET=<strong-random-secret>
NANOSCALE_PUBLIC_BASE_URL=https://<your-public-domain>
NANOSCALE_GITHUB_ENCRYPTION_KEY=<base64-encoded-32-byte-key>
```

Generate encryption key example:

```bash
openssl rand -base64 32
```

If you prefer Python:

```bash
python3 - <<'PY'
import os, base64
print(base64.b64encode(os.urandom(32)).decode())
PY
```

### 11.4 Install the GitHub App on repos

In your GitHub App page:

1. Open **Install App**
2. Choose account/organization
3. Select repositories NanoScale should deploy
4. Include private repositories if you want private-repo deploys

If a repo is not selected in the app installation, NanoScale will not be able to list/deploy it.

Org-level note:

- If you install to an organization, org policy may require owner approval.
- If repositories do not appear in NanoScale, re-open **Install App** and confirm the correct org and repo scope.

### 11.5 Connect from NanoScale dashboard

1. Log in to NanoScale dashboard
2. Open **Projects → New Project**
3. Choose GitHub source mode
4. Click **Connect GitHub** and complete OAuth consent
5. Sync/load repositories and choose a repo/branch
6. Create the project

On successful deploy setup, NanoScale registers a webhook so future `push` events trigger redeploy.

### 11.6 Reverse proxy requirements

Ensure your proxy preserves these headers to the orchestrator:

- `X-GitHub-Event`
- `X-GitHub-Delivery`
- `X-Hub-Signature-256`

If orchestrator is behind NAT/private networking, place a public reverse proxy/LB/tunnel in front so GitHub can reach callback/webhook endpoints.

### 11.7 First-run validation checklist (GitHub integration)

1. In GitHub App settings, confirm callback/webhook URLs exactly match your public NanoScale domain.
2. In NanoScale dashboard, click Connect GitHub and complete OAuth successfully.
3. In New Project, confirm repositories load when you click Sync/Load repos.
4. Create a project from a selected repository and branch.
5. Push a commit to that branch.
6. In GitHub App settings → **Advanced** → **Recent Deliveries**, confirm a `push` delivery to NanoScale returns 2xx.
7. Confirm NanoScale starts a redeploy for that project after the push event.

If step 6 fails, check:

- TLS certificate validity for your public domain
- reverse proxy forwarding for `X-GitHub-Event`, `X-GitHub-Delivery`, and `X-Hub-Signature-256`
- `NANOSCALE_GITHUB_WEBHOOK_SECRET` (or `github.webhook_secret`) exactly matches the App webhook secret

## 12) Troubleshooting

### `SQLite code 14: unable to open database file`

If you run orchestrator with `NANOSCALE_DB_PATH=/opt/nanoscale/data/nanoscale.db`, the process must have write access to `/opt/nanoscale/data`.

Use one of these fixes:

1. Run orchestrator as the `nanoscale` user (recommended for `/opt/nanoscale`):

```bash
sudo -u nanoscale \
./target/release/agent --role orchestrator
```

2. Re-apply expected ownership after install:

```bash
sudo mkdir -p /opt/nanoscale/data
sudo chown -R nanoscale:nanoscale /opt/nanoscale
```

3. For local/dev-only runs, use a DB path in your current directory:

```bash
cp /opt/nanoscale/config.json ./config.json
# edit database_path in ./config.json to ./nanoscale.db
NANOSCALE_CONFIG_PATH=./config.json \
./target/release/agent --role orchestrator
```

### GitHub integration callbacks/webhooks fail

If you enable GitHub integration and private repository deploys, ensure all of the following:

1. Your orchestrator is reachable at a public HTTPS URL.
2. The GitHub App callback URL is:

```text
https://<your-public-domain>/api/integrations/github/callback
```

3. The GitHub App webhook URL is:

```text
https://<your-public-domain>/api/integrations/github/webhook
```

4. The orchestrator has these env vars configured (or matching `github` keys in config):

```bash
NANOSCALE_GITHUB_ENABLED=true
NANOSCALE_GITHUB_APP_ID=<github_app_id>
NANOSCALE_GITHUB_APP_SLUG=<github_app_slug>
NANOSCALE_GITHUB_APP_CLIENT_ID=<github_client_id>
NANOSCALE_GITHUB_APP_CLIENT_SECRET=<github_client_secret>
NANOSCALE_GITHUB_APP_PRIVATE_KEY_PATH=/opt/nanoscale/config/github-app.pem
NANOSCALE_GITHUB_WEBHOOK_SECRET=<strong-random-secret>
NANOSCALE_PUBLIC_BASE_URL=https://<your-public-domain>
NANOSCALE_GITHUB_ENCRYPTION_KEY=<base64-encoded-32-byte-key>
```

Generate encryption key example:

```bash
python3 - <<'PY'
import os, base64
print(base64.b64encode(os.urandom(32)).decode())
PY
```

5. Your reverse proxy preserves these headers:
  - `X-GitHub-Event`
  - `X-GitHub-Delivery`
  - `X-Hub-Signature-256`

6. If your orchestrator is behind NAT/private networking, use one of:
  - Public reverse proxy in front of orchestrator
  - Secure tunnel provider
  - Static public ingress/LB with TLS termination

For private repositories specifically, verify that the GitHub App installation was granted access
to the repository and includes repository permissions for metadata/contents read and webhooks write.
