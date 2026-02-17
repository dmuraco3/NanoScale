# AGENTS.md

This file is for agentic coding assistants working in this repo.
Follow these commands, rules, and style guidelines.

## Quick Map
- Repo is a monorepo: Next.js app in `apps/dashboard`, Rust service in `crates/agent`.
- Toolchains are pinned: Rust 1.93.0 (`rust-toolchain.toml`), Bun 1.2.20 (`.bun-version`).

## Build, Lint, Test Commands

### Root (workspace)
- Install JS deps: `bun install`
- Dev server (dashboard): `bun run dev`
- Build dashboard: `bun run build`
- Lint all: `bun run lint`
- Lint dashboard only: `bun run lint:dashboard`
- Lint Rust only: `bun run lint:rust`
- Rust format check: `bun run format:rust`
- Rust security audit: `bun run audit:rust`

### Dashboard (Next.js, from repo root)
- Dev: `bun run dev`
- Build: `bun run build`
- Start: `cd apps/dashboard && bun run start`
- Lint: `cd apps/dashboard && bun run lint`
- Typecheck: `cd apps/dashboard && bun run typecheck`

### Rust (agent)
- Build debug: `cargo build -p agent`
- Build release: `cargo build --release -p agent`
- Lint (clippy): `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Format check: `cargo fmt --all --check`

### Tests (single test focus)
- Workspace tests: `cargo test --workspace`
- Agent crate tests: `cargo test -p agent`
- Single Rust test by name: `cargo test -p agent <test_name>`
- Single Rust test module: `cargo test -p agent <module_name>`
- Single Rust integration test: `cargo test -p agent --test <file_stem>`

Note: The dashboard currently has lint and typecheck scripts only. No test runner is defined.

## Project Rules (Cursor/Copilot)
- No `.cursor/rules`, `.cursorrules`, or `.github/copilot-instructions.md` found in this repo.

## Code Style Guide (Source of Truth)
See `docs/code-style-guide.md` for the full policy. Highlights below are enforced
and should be followed by agents.

## Core Principles (Language Agnostic)
- High cohesion, low coupling; group by feature/domain.
- 300-line rule: no single Rust/TS file over 300 lines.
- DRY: extract repeated logic (3+ occurrences) but avoid premature abstraction.
- Professionalism: no emojis in UI text, comments, logs, or commits.
- Comments explain why, not what.
- Git commits use Conventional Commits.

## Frontend (Next.js 16 / TypeScript)

### Imports and Structure
- Prefer Server Components by default; push `use client` to leaf components.
- Use the path alias `@/*` per `apps/dashboard/tsconfig.json`.
- Keep React hooks local to client components, avoid cross-file side effects.

### Hooks and Effects
- `useEffect` is prohibited. Do not import or call it.
- For data fetching: use Server Components (async components) for initial load.
- For client polling: use SWR or TanStack Query (if added).
- For external sync (e.g., WebSocket), isolate in a custom hook.

### TypeScript Rigor
- `any` is forbidden (`@typescript-eslint/no-explicit-any`).
- Non-null assertion (`!`) is forbidden (`@typescript-eslint/no-non-null-assertion`).
- Export all interfaces and type definitions.
- Strict mode enabled; respect null/undefined handling.

### Naming Conventions
- Components: PascalCase (e.g., `ServerList.tsx`).
- Functions/variables: camelCase (e.g., `fetchServerStats`).
- Constants: UPPER_SNAKE_CASE (e.g., `MAX_RETRY_COUNT`).

### Formatting
- ESLint config extends `eslint-config-next` (core-web-vitals + typescript).
- No Prettier config found; rely on ESLint and default Next formatting.

## Backend (Rust / Axum)

### Safety and Error Handling
- `unwrap()` is forbidden; use `?` or `expect` with a clear reason.
- Prefer `anyhow` for app-level errors; use `thiserror` for library errors.
- Clippy warnings are denied (workspace lints are strict, pedantic enabled).

### Modularity and Visibility
- Organize modules by domain (e.g., `cluster`, `deployment`, `auth`).
- Keep functions `pub(crate)` unless needed outside a module.

### Async and Runtime
- Use Tokio (`#[tokio::main]`).
- Avoid blocking I/O in async code; use `tokio::task::spawn_blocking`.

### Naming Conventions
- Structs/enums: PascalCase.
- Functions/variables: snake_case.
- Filenames: snake_case.rs.

### Formatting
- `cargo fmt --all --check` is expected in CI.
- Lint alias exists: `cargo lint` in `.cargo/config.toml`.

## Database (SQLite)
- Tables: snake_case, pluralized.
- Columns: snake_case.
- IDs: `TEXT` UUIDv4, not auto-increment ints.
- Index all foreign keys.
- Ensure WAL mode for concurrency.

## Security and Command Execution (Rust)
- Never use `sh -c` or `system()` with user input.
- Use `std::process::Command` with separated args.
- Validate user inputs with allowlist regex (see `docs/technical-specification.md`).

## Practical Tips for Agents
- Keep changes small and aligned to existing structure.
- Follow lint rules; run `bun run lint` and `cargo clippy` when touching both.
- Avoid broad refactors unless explicitly requested.
- If adding new files, respect the 300-line rule and naming conventions.
