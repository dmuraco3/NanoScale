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

```bash
NANOSCALE_DB_PATH=/opt/nanoscale/data/nanoscale.db \
NANOSCALE_ORCHESTRATOR_BIND=0.0.0.0:4000 \
./target/release/agent --role orchestrator
```

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
NANOSCALE_ORCHESTRATOR_URL=http://<ORCHESTRATOR_IP>:4000 \
NANOSCALE_WORKER_IP=<WORKER_IP> \
NANOSCALE_WORKER_NAME=<WORKER_NAME> \
NANOSCALE_WORKER_BIND=0.0.0.0:4000 \
./target/release/agent --join <JOIN_TOKEN>
```

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
