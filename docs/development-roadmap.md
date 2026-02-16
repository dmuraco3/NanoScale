# Development Roadmap: NanoScale

**Version:** 1.0.0  
**Timeline:** 12 Weeks to MVP  
**Goal:** A secure, multi-node PaaS for low-resource VPS hosting.

## Phase 0: Security Foundation & Environment (Weeks 1-2)

**Objective:** Establish the secure runtime environment. The system must be secure before features are added.

### 0.1 Repository Setup

- [x] Initialize Monorepo with turborepo or bun workspaces.
- [x] Set up Rust workspace in `crates/`.
- [x] Set up Next.js app in `apps/dashboard`.
- [x] Linting & Style Enforcement:
	- [x] Configure `cargo clippy` to deny warnings.
	- [x] Configure ESLint with plugin `@next/next/recommended` and strict TypeScript rules.
	- [x] Add a "No-useEffect" lint rule (e.g., `eslint-plugin-react-hooks`).
	- [x] Verify compliance with [code-style-guide.md](code-style-guide.md).
	- [x] Configure `cargo-audit` in CI pipeline to block insecure code.

### 0.2 The Installer Script (`scripts/install.sh`)

Must run as root.

- [x] Dependency Check: Verify `curl`, `git`, `nginx`, `sqlite3` are present. Install if missing.
- [x] User Creation:
	- [x] Create group `nanoscale`.
	- [x] Create system user `nanoscale` with home `/opt/nanoscale` and shell `/bin/false`.
- [x] Directory Structure:
	- [x] Create `/opt/nanoscale/{bin,data,sites,config,logs,tmp}`.
	- [x] Execute `chown -R nanoscale:nanoscale /opt/nanoscale`.
	- [x] Set permissions `0700` on `/opt/nanoscale/sites` (Private).
- [x] Sudoers Configuration:
	- [x] Write file `/etc/sudoers.d/nanoscale` with the exact rules defined in Tech Spec 6.1.
	- [x] Verify syntax with `visudo -c`.
- [x] Firewall:
	- [x] Enable `ufw`.
	- [x] Allow SSH (22), HTTP (80), HTTPS (443).
	- [x] Allow Port 4000 (Internal API) - initially open, later restricted to Cluster IP.

### 0.3 The Agent Skeleton (Rust)

- [x] Initialize `crates/agent`.
- [x] Implement `main.rs` that parses CLI args:
	- [x] `--role orchestrator`: starts DB + API.
	- [x] `--join <token>`: starts Worker logic.
- [x] Implement `PrivilegeWrapper`: a Rust struct that wraps `Command::new("sudo")`.
- [x] Strict Requirement: All system modification calls MUST go through this wrapper.
- [x] Hardcode allowed paths (e.g., `/usr/bin/systemctl`).

## Phase 1: Core Engine & Cluster Protocol (Weeks 3-4)

**Objective:** Enable nodes to talk to each other securely.

### 1.1 Database Implementation

- [x] Define SQLx migrations in `crates/agent/migrations`.
- [x] Implement `DbClient` struct.
- [x] Task: Ensure SQLite runs in WAL mode (`PRAGMA journal_mode=WAL;`) for concurrency.

### 1.2 Cluster Handshake Logic

- [x] Orchestrator Endpoint: Implement `POST /api/cluster/generate-token`.
- [x] Generate a random 32-char string. Store in memory with 10-minute expiry.
- [x] Worker Join Logic:
	- [x] Worker generates a local Keypair (or strong random secret).
	- [x] Worker sends `POST {orchestrator_url}/api/cluster/join` with `{ token, ip, secret_key }`.
- [x] Orchestrator Finalization:
	- [x] Validate token.
	- [x] Create row in `servers` table.
	- [x] Return 200 OK.
- [x] Signature Middleware:
	- [x] Implement Axum middleware `VerifyClusterSignature`.
	- [x] Logic: Recompute `HMAC-SHA256(body + timestamp, stored_secret_key)`.
	- [x] Reject if signature mismatch or timestamp > 30s old.

### 1.3 Internal API (Worker Side)

- [x] Expose `POST /internal/health` (returns CPU/RAM usage).
- [x] Expose `POST /internal/deploy` (placeholder for Phase 3).
- [x] Bind Axum to `0.0.0.0:4000`.

## Phase 2: The Orchestrator Dashboard (Weeks 5-6)

**Objective:** A UI to manage the cluster. Reference: implement strictly according to [ui-specification.md](ui-specification.md).

### 2.1 Dashboard Auth

- [x] Implement `POST /api/auth/setup` (first run only).
- [x] Implement `POST /api/auth/login`.
- [x] Set up `tower-sessions` with SQLite store.
- [x] Create `AuthGuard` React HOC (Higher Order Component) to redirect to login if session missing.

### 2.2 Server Management UI

- [x] List view: table of servers (Name, IP, Status, RAM Usage).
- [x] Add Server:
	- [x] Button "Add Server".
	- [x] Calls API to get Token.
	- [x] Displays one-liner: `curl ... | bash -s -- --join <token> ...`.
	- [x] Polling mechanism to check when new server comes online.

### 2.3 Project Creation UI

- [x] Form: Name, Repo URL, Branch, Env Vars.
- [x] Server Selection: dropdown of available servers (fetched from `servers` table).
- [x] Submission Logic:
	- [x] Frontend POSTs to Orchestrator API.
	- [x] Orchestrator creates DB record.
	- [x] Orchestrator calls `POST /internal/projects` on the target Worker.

## Phase 3: Deployment Pipeline (Weeks 7-8)

**Objective:** Go from Git URL to running process.

### 3.1 The Git Worker (Rust)

- [x] Implement `Git::clone(url, target_dir)`.
- [x] Security: validate URL regex.
- [x] Use `git clone --depth 1`.
- [x] Implement `Git::checkout(branch)`.

### 3.2 The Build System (Local)

- [x] Swap Logic: Check RAM. If <2GB, run `fallocate` (via sudo wrapper).
- [x] Install: execute `bun install --frozen-lockfile` in the build directory.
- [x] Build: execute `bun run build`.
- [x] Artifact Handling:
	- [x] Identify `.next/standalone`.
	- [x] Move artifacts to `/opt/nanoscale/sites/{id}/source`.
	- [x] Crucial: execute `chown -R nanoscale-{id}:nanoscale-{id}` on the source dir.

### 3.3 Systemd Generator

- [x] Create Jinja2 or Rust templates for `.service` and `.socket` files.
- [x] Inject `ProtectSystem=strict`, `NoNewPrivileges=yes`.
- [x] Write files to `/opt/nanoscale/tmp/`.
- [x] Move to `/etc/systemd/system/` via sudo wrapper.
- [x] Execute `systemctl daemon-reload`.

### 3.4 Nginx Generator

- [x] Generate config block with `proxy_pass http://127.0.0.1:{port}`.
- [x] Write to `/etc/nginx/sites-available/`.
- [x] Reload Nginx via sudo wrapper.

### 3.5 Scale-to-Zero Implementation

- [x] Implement `InactivityMonitor` struct in Rust.
- [x] Spawn a Tokio background task (Interval: 60s).
- [x] Loop through active projects:
	- [x] Run `sudo systemctl show --property=ActiveEnterTimestamp ...`.
	- [x] Run `ss -tn src :{port} | wc -l`.
	- [x] Logic: If `connections == 0` AND `uptime > 15m` THEN `sudo systemctl stop {service}`.

## Phase 4: Remote Build & Monetization (Weeks 9-10)

**Objective:** Enable the revenue engine.

### 4.1 Remote Build Client (Agent Side)

- [ ] Modify Deployment Logic to check `type: "remote"`.
- [ ] Implement Zipper: compress repo (ignore `.git`, `node_modules`).
- [ ] Implement Uploader: POST zip to `build.nanoscale.io`.
- [ ] Implement Downloader: fetch artifact zip and unpack.

### 4.2 Cloud Worker (The SaaS Component)

Note: This runs outside the user's VPS.

- [ ] Set up a Fly.io or AWS Lambda handler.
- [ ] Logic:
	- [ ] Receive zip.
	- [ ] `bun install --frozen-lockfile && bun run build`.
	- [ ] Zip output.
	- [ ] Return stream.

## Phase 5: Polish & Agency Features (Weeks 11-12)

### 5.1 Logging

- [ ] Implement SSE endpoint `/api/logs/{id}`.
- [ ] Rust reads `journalctl -u nanoscale-{id} -f -o cat` line-by-line.
- [ ] Stream lines to Frontend XTerm.js component.

### 5.2 White-Labeling

- [ ] Add `settings` table.
- [ ] Allow uploading "Logo URL".
- [ ] Frontend: replace "NanoScale" text with DB value.

### 5.3 Final Security Audit

- [ ] Attempt command injection on all text inputs.
- [ ] Attempt path traversal (`../../`) on repo URLs.
- [ ] Verify unprivileged user cannot read `/etc/shadow`.