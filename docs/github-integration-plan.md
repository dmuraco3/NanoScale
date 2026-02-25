# GitHub Integration Implementation Plan

This document defines the end-to-end implementation plan for GitHub integration in NanoScale, including private repository support, repo selection in New Project flow, and push-triggered redeploys.

## 1) Scope and Success Criteria

- [ ] Add "Integrate with GitHub" from dashboard and complete login/install flow.
- [ ] Allow selecting from user-accessible GitHub repositories (public + private) in New Project form.
- [ ] Deploy projects from selected repositories using orchestrator-managed credentials.
- [ ] Install and manage webhook subscriptions for selected repos.
- [ ] Trigger project redeploy on valid push events.
- [ ] Keep all business logic in orchestrator backend.
- [ ] Keep app self-hosted (no mandatory NanoScale-managed SaaS dependency).

## 2) High-Level Architecture

- [ ] Use **GitHub App** integration model as primary provider.
- [ ] Keep manual `repo_url` deploy path as fallback for non-integrated users.
- [ ] Add orchestrator-managed integration state (account link, installations, repo mappings, webhook metadata).
- [ ] Use short-lived installation access tokens for Git operations and GitHub API calls.
- [ ] Expose orchestrator webhook endpoint for GitHub push events with signature verification.
- [ ] Keep dashboard as client/UI only; no GitHub secrets in frontend.

## 3) GitHub App Permission Matrix (Required)

### Repository permissions

- [ ] `Contents: Read-only` (clone/fetch and metadata for deployment source).
- [ ] `Metadata: Read-only` (list repos and identify repository details).
- [ ] `Webhooks: Read & write` (create/update/delete repo webhooks).

### Account permissions

- [ ] `Email addresses: Read-only` (optional; only if needed for account display).

### Events

- [ ] Subscribe to `push` event.
- [ ] (Optional future) Subscribe to `repository` events for rename/archive handling.

### Access model

- [ ] Ensure installation supports selecting **All repositories** or **Only selected repositories**.
- [ ] Support private repositories included in installation scope.

## 4) Self-Hosted Runtime Requirements

- [ ] Configure GitHub App credentials on orchestrator host:
  - [ ] `NANOSCALE_GITHUB_APP_ID`
  - [ ] `NANOSCALE_GITHUB_APP_CLIENT_ID`
  - [ ] `NANOSCALE_GITHUB_APP_CLIENT_SECRET`
  - [ ] `NANOSCALE_GITHUB_APP_PRIVATE_KEY_PATH` (or secure inline secret variant)
  - [ ] `NANOSCALE_GITHUB_WEBHOOK_SECRET`
  - [ ] `NANOSCALE_PUBLIC_BASE_URL` (public HTTPS URL used for callback/webhook)
- [ ] Enforce startup validation: fail fast with clear errors if GitHub integration is enabled but required config is missing.
- [ ] Require HTTPS for callback and webhook URLs in non-dev environments.
- [ ] Document reverse proxy/tunnel options when orchestrator is behind NAT.

## 5) Data Model and Migrations

## 5.1 New tables

- [ ] Add `github_user_links` table:
  - [ ] `id TEXT PRIMARY KEY`
  - [ ] `local_user_id TEXT NOT NULL` (FK to `users.id`)
  - [ ] `github_user_id INTEGER NOT NULL`
  - [ ] `github_login TEXT NOT NULL`
  - [ ] `access_token_encrypted TEXT NOT NULL`
  - [ ] `refresh_token_encrypted TEXT` (nullable if not issued)
  - [ ] `token_expires_at DATETIME` (nullable if non-expiring)
  - [ ] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
  - [ ] `updated_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [ ] Add unique index on (`local_user_id`).
- [ ] Add unique index on (`github_user_id`).

- [ ] Add `github_installations` table:
  - [ ] `id TEXT PRIMARY KEY`
  - [ ] `local_user_id TEXT NOT NULL`
  - [ ] `installation_id INTEGER NOT NULL`
  - [ ] `account_login TEXT NOT NULL`
  - [ ] `account_type TEXT NOT NULL` (User/Organization)
  - [ ] `target_type TEXT NOT NULL`
  - [ ] `target_id INTEGER NOT NULL`
  - [ ] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
  - [ ] `updated_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [ ] Add unique index on (`installation_id`).
- [ ] Add index on (`local_user_id`).

- [ ] Add `github_repositories` table:
  - [ ] `id TEXT PRIMARY KEY`
  - [ ] `installation_id INTEGER NOT NULL`
  - [ ] `repo_id INTEGER NOT NULL` (GitHub repository id)
  - [ ] `node_id TEXT NOT NULL`
  - [ ] `owner_login TEXT NOT NULL`
  - [ ] `name TEXT NOT NULL`
  - [ ] `full_name TEXT NOT NULL`
  - [ ] `default_branch TEXT NOT NULL`
  - [ ] `is_private BOOLEAN NOT NULL`
  - [ ] `html_url TEXT NOT NULL`
  - [ ] `clone_url TEXT NOT NULL`
  - [ ] `archived BOOLEAN NOT NULL DEFAULT 0`
  - [ ] `disabled BOOLEAN NOT NULL DEFAULT 0`
  - [ ] `last_synced_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [ ] Add unique index on (`repo_id`).
- [ ] Add index on (`installation_id`).
- [ ] Add index on (`full_name`).

- [ ] Add `project_github_links` table:
  - [ ] `id TEXT PRIMARY KEY`
  - [ ] `project_id TEXT NOT NULL` (FK to `projects.id`, unique)
  - [ ] `installation_id INTEGER NOT NULL`
  - [ ] `repo_id INTEGER NOT NULL`
  - [ ] `repo_node_id TEXT NOT NULL`
  - [ ] `owner_login TEXT NOT NULL`
  - [ ] `repo_name TEXT NOT NULL`
  - [ ] `full_name TEXT NOT NULL`
  - [ ] `default_branch TEXT NOT NULL`
  - [ ] `selected_branch TEXT NOT NULL`
  - [ ] `webhook_id INTEGER`
  - [ ] `webhook_secret_encrypted TEXT NOT NULL`
  - [ ] `active BOOLEAN NOT NULL DEFAULT 1`
  - [ ] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
  - [ ] `updated_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [ ] Add unique index on (`project_id`).
- [ ] Add index on (`repo_id`, `selected_branch`).

- [ ] Add `github_webhook_deliveries` table (idempotency + audit):
  - [ ] `id TEXT PRIMARY KEY`
  - [ ] `delivery_id TEXT NOT NULL` (`X-GitHub-Delivery`)
  - [ ] `event_type TEXT NOT NULL`
  - [ ] `repo_id INTEGER`
  - [ ] `ref TEXT`
  - [ ] `head_commit TEXT`
  - [ ] `handled BOOLEAN NOT NULL DEFAULT 0`
  - [ ] `status_code INTEGER`
  - [ ] `error_message TEXT`
  - [ ] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [ ] Add unique index on (`delivery_id`).

## 5.2 Existing table updates

- [ ] Keep existing `projects.repo_url` for backward compatibility.
- [ ] Add optional `source_provider TEXT` to `projects` (`manual` | `github`).
- [ ] Add optional `source_repo_id INTEGER` to `projects` for quick lookups.

## 5.3 Data protection

- [ ] Encrypt all long-lived sensitive tokens/secrets at rest.
- [ ] Store encryption key outside DB (env/config only).
- [ ] Add key rotation plan with re-encryption migration steps.

## 6) Orchestrator API Contracts

## 6.1 Integration/auth endpoints

- [ ] `POST /api/integrations/github/start`
  - [ ] Requires authenticated NanoScale session.
  - [ ] Returns redirect URL for GitHub App/OAuth handshake with signed `state`.
- [ ] `GET /api/integrations/github/callback`
  - [ ] Validates `state`, exchanges code, persists account link.
  - [ ] Redirects back to dashboard integration status page.
- [ ] `GET /api/integrations/github/status`
  - [ ] Returns connected/disconnected status, account login, installation summary.
- [ ] `POST /api/integrations/github/disconnect`
  - [ ] Revokes local link, clears tokens, deactivates repo mappings.

## 6.2 Repository discovery endpoints

- [ ] `GET /api/integrations/github/installations`
  - [ ] Returns installations available to current user.
- [ ] `GET /api/integrations/github/repos?installation_id=...&cursor=...&query=...`
  - [ ] Returns paginated repository list including private repos in scope.
- [ ] `POST /api/integrations/github/repos/sync`
  - [ ] Triggers on-demand refresh from GitHub API to local cache.

## 6.3 Project create/redeploy integration

- [ ] Extend `POST /api/projects` payload to support provider-backed source:
  - [ ] Existing manual fields remain valid.
  - [ ] New optional object `github_source`:
    - [ ] `installation_id: number`
    - [ ] `repo_id: number`
    - [ ] `selected_branch: string`
- [ ] Validate either manual source OR `github_source` is supplied.
- [ ] On create from GitHub source:
  - [ ] Resolve canonical repo metadata from cached/validated GitHub data.
  - [ ] Persist `project_github_links`.
  - [ ] Create or reconcile webhook.

## 6.4 Webhook ingest endpoint

- [ ] `POST /api/integrations/github/webhook`
  - [ ] Public endpoint (no app session required).
  - [ ] Validate `X-Hub-Signature-256` against configured webhook secret.
  - [ ] Validate expected event headers (`X-GitHub-Event`, `X-GitHub-Delivery`).
  - [ ] Dedupe by delivery ID.
  - [ ] Handle `push` events; map repo+branch to project(s).
  - [ ] Trigger redeploy flow via orchestrator business logic.
  - [ ] Return deterministic status codes for observability.

## 7) Dashboard UX Plan

- [ ] Add integration entry point in project creation path:
  - [ ] "Integrate with GitHub" button.
  - [ ] Connected state badge/user login display.
  - [ ] Disconnect action.
- [ ] In New Project form:
  - [ ] Add source mode selector (`Manual URL` | `GitHub Repository`).
  - [ ] For GitHub mode, show installation selector + repo selector + branch selector.
  - [ ] Show private repo indicator in dropdown/list.
  - [ ] Keep existing manual repo URL inputs unchanged for fallback.
- [ ] Ensure no GitHub secret/token is exposed client-side.
- [ ] Preserve no-`useEffect` constraint; use server actions/routes or existing patterns.

## 8) Deployment and Webhook Business Logic (Orchestrator-Owned)

- [ ] During deployment from GitHub source:
  - [ ] Request short-lived installation token server-side.
  - [ ] Construct authenticated clone URL or git credential helper strategy.
  - [ ] Clone selected branch.
- [ ] After successful project creation/deploy:
  - [ ] Create webhook on selected repo for push events.
  - [ ] If webhook already exists, update/reuse idempotently.
  - [ ] Persist webhook ID + encrypted secret in `project_github_links`.
- [ ] On project delete:
  - [ ] Remove or disable webhook if no other project depends on same repo/branch rule.
  - [ ] Clean up mapping records.
- [ ] On redeploy from webhook:
  - [ ] Enforce per-project lock (avoid concurrent redeploy races).
  - [ ] Debounce push bursts (short cooldown window).
  - [ ] Log deployment correlation metadata (delivery ID, commit SHA).

## 9) Private Repository Access Controls

- [ ] Restrict actions to repositories explicitly accessible through the user’s installation.
- [ ] Never persist installation access tokens in plaintext.
- [ ] Use short TTL tokens only for immediate API/clone operations.
- [ ] Redact private repo URLs/tokens from logs and errors.
- [ ] Validate clone source against trusted GitHub host allowlist.
- [ ] Prevent repo spoofing by checking immutable `repo_id`/`node_id` rather than name alone.

## 10) Security and Hardening

- [ ] Add CSRF protection for start/callback (`state` binding + expiry).
- [ ] Validate callback origin and expected app identifiers.
- [ ] Add request size limits and rate limiting on webhook endpoint.
- [ ] Verify webhook signatures with constant-time compare.
- [ ] Add replay protection via `X-GitHub-Delivery` dedupe table.
- [ ] Add strict input validation for branch names and IDs.
- [ ] Add structured audit logs for auth, webhook, and redeploy outcomes.

## 11) Testing Plan

## 11.1 Backend tests (Rust)

- [ ] Unit tests for provider config validation and startup failure modes.
- [ ] Unit tests for OAuth/App callback state validation.
- [ ] Unit tests for webhook signature verification and replay rejection.
- [ ] Unit tests for repo-to-project mapping resolution by `repo_id + branch`.
- [ ] Unit tests for lock/debounce behavior on repeated push events.
- [ ] Integration tests for create project from GitHub source and webhook-triggered redeploy.

## 11.2 Dashboard tests/verification

- [ ] Typecheck + lint after UI changes.
- [ ] Verify source mode switching and payload correctness.
- [ ] Verify private repo appears and can be selected.
- [ ] Verify graceful fallback when integration is disconnected.

## 11.3 Manual E2E checks

- [ ] Connect GitHub account.
- [ ] Select private repo and deploy successfully.
- [ ] Push commit to selected branch and confirm redeploy.
- [ ] Push to non-selected branch and confirm no redeploy.
- [ ] Delete project and confirm webhook cleanup behavior.

## 12) Operational Documentation Updates

- [ ] Update installation guide with GitHub App creation/config steps.
- [ ] Add callback and webhook URL examples for self-hosted domains.
- [ ] Add troubleshooting for webhook delivery failures and signature mismatch.
- [ ] Add secret management guidance and rotation runbook.
- [ ] Add reverse proxy requirements for preserving headers and HTTPS.

## 13) Rollout Strategy

- [ ] Phase A: schema + config + read-only status endpoints.
- [ ] Phase B: auth/connect + repo listing (no deploy yet).
- [ ] Phase C: project creation from GitHub source.
- [ ] Phase D: webhook create + push-triggered redeploy.
- [ ] Phase E: hardening, docs, and release validation.

## 14) Acceptance Checklist

- [ ] User can connect GitHub from dashboard.
- [ ] User can see and select private repos they granted.
- [ ] Project deploy succeeds from selected private repo.
- [ ] Webhook is installed automatically for integrated project.
- [ ] Push to selected branch triggers exactly one redeploy.
- [ ] Invalid signature/replayed delivery is rejected.
- [ ] Manual repo URL deploy remains functional.
- [ ] Self-hosted setup instructions are complete and reproducible.
