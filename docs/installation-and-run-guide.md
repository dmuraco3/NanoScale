# NanoScale Installation & Run Guide

This guide covers the current MVP flow in this repository:

- Install baseline host prerequisites with `scripts/install.sh`
- Run the Rust agent as an Orchestrator
- Run the Next.js dashboard
- Add Worker nodes with a join token

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

## 3) Install Baseline System Dependencies

Run the installer as root on each machine.

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
- Installs sudoers file from `scripts/security/sudoers.d/nanoscale`
- Enables firewall rules for ports `22`, `80`, `443`, `4000`

## 4) Build the Agent Binary

From repo root:

```bash
cargo build --release -p agent
```

Binary path:

- `target/release/agent`

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
./target/release/agent --role orchestrator
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

- This repo currently documents and supports running processes directly from the shell.
- If you want process persistence/restarts across reboots, add system services (systemd units) for:
  - orchestrator agent process
  - dashboard process (or reverse-proxy to a managed runtime)
- Keep Rust/Bun versions pinned to avoid CI/local mismatch.

## 10) Troubleshooting

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
