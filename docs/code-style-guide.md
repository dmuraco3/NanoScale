# Code Style Guide: NanoScale

**Version:** 1.0.0  
**Enforcement:** CI/CD Pipeline (ESLint, Clippy, Prettier, Cargo Fmt)

## 1. Core Principles (Language Agnostic)

### 1.1 High Cohesion, Low Coupling

- **Cohesion:** Code that changes together should stay together. Group files by feature/domain (e.g., `deployment/`) rather than by type (e.g., `controllers/`, `services/`).
- **Coupling:** Modules should interact through strict, minimal interfaces. Avoid global state. Avoid circular dependencies.

### 1.2 The 300-Line Rule

- No single file (Rust or TypeScript) may exceed 300 lines.
- If a file approaches this limit, refactor immediately by extracting sub-components, helper functions, or logic into separate modules.

### 1.3 DRY (Don't Repeat Yourself)

- If logic is repeated three times, extract it into a shared utility or component.
- **Exception:** Do not abstract prematurely. A little duplication is better than the wrong abstraction.

### 1.4 Professionalism

- **No Emojis:** Do not use emojis in UI text, comments, git commits, or console logs. Use icons (Lucide React) for UI.
- **Comments:** Explain why, not what. Code should be self-documenting.
- **Git:** Use Conventional Commits.

```text
feat: add server joining logic
fix: resolve socket activation timeout
refactor: extract sidebar component
```

## 2. Frontend Style (Next.js 16 / TypeScript)

### 2.1 The "No useEffect" Rule

- **Strict Prohibition:** Do not use `useEffect` for data fetching or derived state.
- **Data Fetching:** Use Server Components (async components) for initial data. Use SWR or TanStack Query for client-side polling.
- **Event Handling:** Put logic in event handlers (`onClick`), not effects.
- **Synchronization:** If you must synchronize with an external system (e.g., setting up a WebSocket), isolate it in a custom hook.

### 2.2 Component Composition

- **Atomic Design:** Break complex UIs into small, single-responsibility components.
- **Pattern:**
  - **Bad:** A 500-line `DashboardPage.tsx` with 10 `useState` hooks.
  - **Good:** `DashboardPage` fetches data and passes it to `<MetricsGrid />`, `<ActivityFeed />`, and `<ServerStatus />`.
- **Server vs Client:** Keep components Server Components by default. Push `use client` down the tree to the leaves (buttons, inputs) that actually need interactivity.

### 2.3 TypeScript Rigor

- **No `any`:** The `any` type is strictly forbidden.
- **Export Interfaces:** All interface and type definitions must be exported.
- **Prefer named functions over arrow functions:** Use `function foo() {}` (or `async function foo() {}`) instead of `const foo = () => {}` for named/standalone functions.

```ts
// Good
export interface ServerProps {
  id: string;
  status: 'online' | 'offline';
}
```

- **Strict Null Checks:** Handled via `tsconfig.json`. Do not use non-null assertions (`!`) unless absolutely necessary.

### 2.4 Naming Conventions

- **Components:** PascalCase (e.g., `ServerList.tsx`)
- **Functions/Variables:** camelCase (e.g., `fetchServerStats`)
- **Constants:** UPPER_SNAKE_CASE (e.g., `MAX_RETRY_COUNT`)

### 2.5 Next.js Server Actions

- **Server Actions must live in separate files.** Define actions in dedicated modules (e.g., `app/.../actions.ts` or `lib/*-actions.ts`) and import them where needed.
- **Never define Server Actions inside React components.** Do not create `async function action()` bodies within component files or component scopes.
- **Keep actions close to the domain.** Place the action module next to the route segment or feature it serves, rather than in a global catch-all.

## 3. Backend Style (Rust / Axum)

### 3.1 Safety & Error Handling

- **No `unwrap()`:** Never use `.unwrap()` in production code. It causes panics.
- **Use:** `.expect("Reason why this should not fail")` or propagate errors with `?`.
- **Error Types:** Use `thiserror` for library errors and `anyhow` for application binaries.
- **Clippy:** The codebase must pass `cargo clippy -- -D warnings`. Warnings are treated as errors.

### 3.2 Modularity

- **Domain Driven:** Organize crates and modules by business logic (`cluster`, `deployment`, `auth`).
- **Public Interfaces:** Only make functions `pub` if they are used outside the module. Use `pub(crate)` for internal sharing.

### 3.3 Async/Await

- **Tokio:** Use `#[tokio::main]` for the entry point.
- **Blocking:** Never perform blocking I/O (file reads, heavy CPU) in an async function. Use `tokio::task::spawn_blocking`.

### 3.4 Naming Conventions

- **Structs/Enums:** PascalCase
- **Variables/Functions:** snake_case
- **Files:** snake_case.rs

## 4. Database Style (SQL / SQLite)

### 4.1 Schema Design

- **Tables:** snake_case, pluralized (e.g., `servers`, `projects`, `users`)
- **Columns:** snake_case (e.g., `created_at`, `repo_url`)
- **Primary Keys:** Always use `TEXT` (UUID v4) for IDs to allow easy migration/merging later. Do not use auto-increment integers.

### 4.2 Performance

- **Indexes:** All foreign keys (`server_id`) must be indexed.
- **WAL Mode:** Ensure SQLite is running in Write-Ahead Logging mode for concurrency.