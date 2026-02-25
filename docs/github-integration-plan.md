# GitHub Integration Implementation Plan

This document defines the end-to-end implementation plan for GitHub integration in NanoScale, including private repository support, repo selection in New Project flow, and push-triggered redeploys.

## 1) Scope and Success Criteria

- [x] Add "Integrate with GitHub" from dashboard and complete login/install flow.
- [x] Allow selecting from user-accessible GitHub repositories (public + private) in New Project form.
- [x] Deploy projects from selected repositories using orchestrator-managed credentials.
- [x] Install and manage webhook subscriptions for selected repos.
- [x] Trigger project redeploy on valid push events.
- [x] Keep all business logic in orchestrator backend.
- [x] Keep app self-hosted (no mandatory NanoScale-managed SaaS dependency).

## 2) High-Level Architecture

- [x] Use **GitHub App** integration model as primary provider.
- [x] Keep manual `repo_url` deploy path as fallback for non-integrated users.
- [x] Add orchestrator-managed integration state (account link, installations, repo mappings, webhook metadata).
- [x] Use short-lived installation access tokens for Git operations and GitHub API calls.
- [x] Expose orchestrator webhook endpoint for GitHub push events with signature verification.
- [x] Keep dashboard as client/UI only; no GitHub secrets in frontend.

## 3) GitHub App Permission Matrix (Required)

### Repository permissions

- [x] `Contents: Read-only` (clone/fetch and metadata for deployment source).
- [x] `Metadata: Read-only` (list repos and identify repository details).
- [x] `Webhooks: Read & write` (create/update/delete repo webhooks).

### Account permissions

- [x] `Email addresses: Read-only` (optional; only if needed for account display).

### Events

- [x] Subscribe to `push` event.
- [x] (Optional future) Subscribe to `repository` events for rename/archive handling.

### Access model

- [x] Ensure installation supports selecting **All repositories** or **Only selected repositories**.
- [x] Support private repositories included in installation scope.

## 4) Self-Hosted Runtime Requirements

- [x] Configure GitHub App credentials on orchestrator host:
  - [x] `NANOSCALE_GITHUB_APP_ID`
  - [x] `NANOSCALE_GITHUB_APP_CLIENT_ID`
  - [x] `NANOSCALE_GITHUB_APP_CLIENT_SECRET`
  - [x] `NANOSCALE_GITHUB_APP_PRIVATE_KEY_PATH` (or secure inline secret variant)
  - [x] `NANOSCALE_GITHUB_WEBHOOK_SECRET`
  - [x] `NANOSCALE_PUBLIC_BASE_URL` (public HTTPS URL used for callback/webhook)
- [x] Enforce startup validation: fail fast with clear errors if GitHub integration is enabled but required config is missing.
- [x] Require HTTPS for callback and webhook URLs in non-dev environments.
- [x] Document reverse proxy/tunnel options when orchestrator is behind NAT.

## 5) Data Model and Migrations

## 5.1 New tables

- [x] Add `github_user_links` table:
  - [x] `id TEXT PRIMARY KEY`
  - [x] `local_user_id TEXT NOT NULL` (FK to `users.id`)
  - [x] `github_user_id INTEGER NOT NULL`
  - [x] `github_login TEXT NOT NULL`
  - [x] `access_token_encrypted TEXT NOT NULL`
  - [x] `refresh_token_encrypted TEXT` (nullable if not issued)
  - [x] `token_expires_at DATETIME` (nullable if non-expiring)
  - [x] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
  - [x] `updated_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [x] Add unique index on (`local_user_id`).
- [x] Add unique index on (`github_user_id`).

- [x] Add `github_installations` table:
  - [x] `id TEXT PRIMARY KEY`
  - [x] `local_user_id TEXT NOT NULL`
  - [x] `installation_id INTEGER NOT NULL`
  - [x] `account_login TEXT NOT NULL`
  - [x] `account_type TEXT NOT NULL` (User/Organization)
  - [x] `target_type TEXT NOT NULL`
  - [x] `target_id INTEGER NOT NULL`
  - [x] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
  - [x] `updated_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [x] Add unique index on (`installation_id`).
- [x] Add index on (`local_user_id`).

- [x] Add `github_repositories` table:
  - [x] `id TEXT PRIMARY KEY`
  - [x] `installation_id INTEGER NOT NULL`
  - [x] `repo_id INTEGER NOT NULL` (GitHub repository id)
  - [x] `node_id TEXT NOT NULL`
  - [x] `owner_login TEXT NOT NULL`
  - [x] `name TEXT NOT NULL`
  - [x] `full_name TEXT NOT NULL`
  - [x] `default_branch TEXT NOT NULL`
  - [x] `is_private BOOLEAN NOT NULL`
  - [x] `html_url TEXT NOT NULL`
  - [x] `clone_url TEXT NOT NULL`
  - [x] `archived BOOLEAN NOT NULL DEFAULT 0`
  - [x] `disabled BOOLEAN NOT NULL DEFAULT 0`
  - [x] `last_synced_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [x] Add unique index on (`repo_id`).
- [x] Add index on (`installation_id`).
- [x] Add index on (`full_name`).

- [x] Add `project_github_links` table:
  - [x] `id TEXT PRIMARY KEY`
  - [x] `project_id TEXT NOT NULL` (FK to `projects.id`, unique)
  - [x] `installation_id INTEGER NOT NULL`
  - [x] `repo_id INTEGER NOT NULL`
  - [x] `repo_node_id TEXT NOT NULL`
  - [x] `owner_login TEXT NOT NULL`
  - [x] `repo_name TEXT NOT NULL`
  - [x] `full_name TEXT NOT NULL`
  - [x] `default_branch TEXT NOT NULL`
  - [x] `selected_branch TEXT NOT NULL`
  - [x] `webhook_id INTEGER`
  - [x] `webhook_secret_encrypted TEXT NOT NULL`
  - [x] `active BOOLEAN NOT NULL DEFAULT 1`
  - [x] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
  - [x] `updated_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [x] Add unique index on (`project_id`).
- [x] Add index on (`repo_id`, `selected_branch`).

- [x] Add `github_webhook_deliveries` table (idempotency + audit):
  - [x] `id TEXT PRIMARY KEY`
  - [x] `delivery_id TEXT NOT NULL` (`X-GitHub-Delivery`)
  - [x] `event_type TEXT NOT NULL`
  - [x] `repo_id INTEGER`
  - [x] `ref TEXT`
  - [x] `head_commit TEXT`
  - [x] `handled BOOLEAN NOT NULL DEFAULT 0`
  - [x] `status_code INTEGER`
  - [x] `error_message TEXT`
  - [x] `created_at DATETIME DEFAULT CURRENT_TIMESTAMP`
- [x] Add unique index on (`delivery_id`).

## 5.2 Existing table updates

- [x] Keep existing `projects.repo_url` for backward compatibility.
- [x] Add optional `source_provider TEXT` to `projects` (`manual` | `github`).
- [x] Add optional `source_repo_id INTEGER` to `projects` for quick lookups.

## 5.3 Data protection

- [x] Encrypt all long-lived sensitive tokens/secrets at rest.
- [x] Store encryption key outside DB (env/config only).
- [x] Add key rotation plan with re-encryption migration steps.

## 6) Orchestrator API Contracts

## 6.1 Integration/auth endpoints

- [x] `POST /api/integrations/github/start`
  - [x] Requires authenticated NanoScale session.
  - [x] Returns redirect URL for GitHub App/OAuth handshake with signed `state`.
- [x] `GET /api/integrations/github/callback`
  - [x] Validates `state`, exchanges code, persists account link.
  - [x] Redirects back to dashboard integration status page.
- [x] `GET /api/integrations/github/status`
  - [x] Returns connected/disconnected status, account login, installation summary.
- [x] `POST /api/integrations/github/disconnect`
  - [x] Revokes local link, clears tokens, deactivates repo mappings.

## 6.2 Repository discovery endpoints

- [x] `GET /api/integrations/github/installations`
  - [x] Returns installations available to current user.
- [x] `GET /api/integrations/github/repos?installation_id=...&cursor=...&query=...`
  - [x] Returns paginated repository list including private repos in scope.
- [x] `POST /api/integrations/github/repos/sync`
  - [x] Triggers on-demand refresh from GitHub API to local cache.

## 6.3 Project create/redeploy integration

- [x] Extend `POST /api/projects` payload to support provider-backed source:
  - [x] Existing manual fields remain valid.
  - [x] New optional object `github_source`:
    - [x] `installation_id: number`
    - [x] `repo_id: number`
    - [x] `selected_branch: string`
- [x] Validate either manual source OR `github_source` is supplied.
- [x] On create from GitHub source:
  - [x] Resolve canonical repo metadata from cached/validated GitHub data.
  - [x] Persist `project_github_links`.
  - [x] Create or reconcile webhook.

## 6.4 Webhook ingest endpoint

- [x] `POST /api/integrations/github/webhook`
  - [x] Public endpoint (no app session required).
  - [x] Validate `X-Hub-Signature-256` against configured webhook secret.
  - [x] Validate expected event headers (`X-GitHub-Event`, `X-GitHub-Delivery`).
  - [x] Dedupe by delivery ID.
  - [x] Handle `push` events; map repo+branch to project(s).
  - [x] Trigger redeploy flow via orchestrator business logic.
  - [x] Return deterministic status codes for observability.

## 7) Dashboard UX Plan

- [x] Add integration entry point in project creation path:
  - [x] "Integrate with GitHub" button.
  - [x] Connected state badge/user login display.
  - [x] Disconnect action.
- [x] In New Project form:
  - [x] Add source mode selector (`Manual URL` | `GitHub Repository`).
  - [x] For GitHub mode, show installation selector + repo selector + branch selector.
  - [x] Show private repo indicator in dropdown/list.
  - [x] Keep existing manual repo URL inputs unchanged for fallback.
- [x] Ensure no GitHub secret/token is exposed client-side.
- [x] Preserve no-`useEffect` constraint; use server actions/routes or existing patterns.

## 8) Deployment and Webhook Business Logic (Orchestrator-Owned)

- [x] During deployment from GitHub source:
  - [x] Request short-lived installation token server-side.
  - [x] Construct authenticated clone URL or git credential helper strategy.
  - [x] Clone selected branch.
- [x] After successful project creation/deploy:
  - [x] Create webhook on selected repo for push events.
  - [x] If webhook already exists, update/reuse idempotently.
  - [x] Persist webhook ID + encrypted secret in `project_github_links`.
- [x] On project delete:
  - [x] Remove or disable webhook if no other project depends on same repo/branch rule.
  - [x] Clean up mapping records.
- [x] On redeploy from webhook:
  - [x] Enforce per-project lock (avoid concurrent redeploy races).
  - [x] Debounce push bursts (short cooldown window).
  - [x] Log deployment correlation metadata (delivery ID, commit SHA).

## 9) Private Repository Access Controls

- [x] Restrict actions to repositories explicitly accessible through the user’s installation.
- [x] Never persist installation access tokens in plaintext.
- [x] Use short TTL tokens only for immediate API/clone operations.
- [x] Redact private repo URLs/tokens from logs and errors.
- [x] Validate clone source against trusted GitHub host allowlist.
- [x] Prevent repo spoofing by checking immutable `repo_id`/`node_id` rather than name alone.

## 10) Security and Hardening

- [x] Add CSRF protection for start/callback (`state` binding + expiry).
- [x] Validate callback origin and expected app identifiers.
- [x] Add request size limits and rate limiting on webhook endpoint.
- [x] Verify webhook signatures with constant-time compare.
- [x] Add replay protection via `X-GitHub-Delivery` dedupe table.
- [x] Add strict input validation for branch names and IDs.
- [x] Add structured audit logs for auth, webhook, and redeploy outcomes.

## 11) Testing Plan

## 11.1 Backend tests (Rust)

- [x] Unit tests for provider config validation and startup failure modes.
- [x] Unit tests for OAuth/App callback state validation.
- [x] Unit tests for webhook signature verification and replay rejection.
- [x] Unit tests for repo-to-project mapping resolution by `repo_id + branch`.
- [x] Unit tests for lock/debounce behavior on repeated push events.
- [x] Integration tests for create project from GitHub source and webhook-triggered redeploy.

## 11.2 Dashboard tests/verification

- [x] Typecheck + lint after UI changes.
- [x] Verify source mode switching and payload correctness.
- [x] Verify private repo appears and can be selected.
- [x] Verify graceful fallback when integration is disconnected.

## 11.3 Manual E2E checks

- [x] Connect GitHub account.
- [x] Select private repo and deploy successfully.
- [x] Push commit to selected branch and confirm redeploy.
- [x] Push to non-selected branch and confirm no redeploy.
- [x] Delete project and confirm webhook cleanup behavior.

## 12) Operational Documentation Updates

- [x] Update installation guide with GitHub App creation/config steps.
- [x] Add callback and webhook URL examples for self-hosted domains.
- [x] Add troubleshooting for webhook delivery failures and signature mismatch.
- [x] Add secret management guidance and rotation runbook.
- [x] Add reverse proxy requirements for preserving headers and HTTPS.

## 13) Rollout Strategy

- [x] Phase A: schema + config + read-only status endpoints.
- [x] Phase B: auth/connect + repo listing (no deploy yet).
- [x] Phase C: project creation from GitHub source.
- [x] Phase D: webhook create + push-triggered redeploy.
- [x] Phase E: hardening, docs, and release validation.

## 14) Acceptance Checklist

- [x] User can connect GitHub from dashboard.
- [x] User can see and select private repos they granted.
- [x] Project deploy succeeds from selected private repo.
- [x] Webhook is installed automatically for integrated project.
- [x] Push to selected branch triggers exactly one redeploy.
- [x] Invalid signature/replayed delivery is rejected.
- [x] Manual repo URL deploy remains functional.
- [x] Self-hosted setup instructions are complete and reproducible.
