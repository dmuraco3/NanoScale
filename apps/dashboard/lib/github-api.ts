import { clientApiBaseUrl } from "@/lib/api-base-url";

export interface GitHubStatus {
  enabled: boolean;
  configured: boolean;
  connected: boolean;
  github_login: string | null;
  app_install_url: string | null;
}

export interface GitHubInstallation {
  installation_id: number;
  account_login: string;
  account_type: string;
}

export interface GitHubRepository {
  installation_id: number;
  repo_id: number;
  owner_login: string;
  name: string;
  full_name: string;
  default_branch: string;
  is_private: boolean;
  clone_url: string;
}

export async function fetchGitHubStatus(): Promise<GitHubStatus | null> {
  const response = await fetch(`${clientApiBaseUrl()}/api/integrations/github/status`, {
    credentials: "include",
    cache: "no-store",
  });

  if (!response.ok) {
    return null;
  }

  return (await response.json()) as GitHubStatus;
}

export async function startGitHubIntegration(): Promise<string> {
  const response = await fetch(`${clientApiBaseUrl()}/api/integrations/github/start`, {
    method: "POST",
    credentials: "include",
  });

  if (!response.ok) {
    throw new Error("Unable to start GitHub integration");
  }

  const payload = (await response.json()) as { redirect_url: string };
  return payload.redirect_url;
}

export async function disconnectGitHubIntegration(): Promise<void> {
  const response = await fetch(`${clientApiBaseUrl()}/api/integrations/github/disconnect`, {
    method: "POST",
    credentials: "include",
  });

  if (!response.ok) {
    throw new Error("Unable to disconnect GitHub integration");
  }
}

export async function fetchGitHubInstallations(): Promise<GitHubInstallation[]> {
  const response = await fetch(`${clientApiBaseUrl()}/api/integrations/github/installations`, {
    credentials: "include",
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error("Unable to load GitHub installations");
  }

  return (await response.json()) as GitHubInstallation[];
}

export async function syncGitHubRepositories(installationId: number): Promise<void> {
  const response = await fetch(`${clientApiBaseUrl()}/api/integrations/github/repos/sync`, {
    method: "POST",
    credentials: "include",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ installation_id: installationId }),
  });

  if (!response.ok) {
    throw new Error("Unable to sync GitHub repositories");
  }
}

export async function fetchGitHubRepositories(
  installationId: number,
  query?: string,
): Promise<GitHubRepository[]> {
  const params = new URLSearchParams();
  params.set("installation_id", String(installationId));
  if (query && query.trim().length > 0) {
    params.set("query", query.trim());
  }

  const response = await fetch(
    `${clientApiBaseUrl()}/api/integrations/github/repos?${params.toString()}`,
    {
      credentials: "include",
      cache: "no-store",
    },
  );

  if (!response.ok) {
    throw new Error("Unable to load GitHub repositories");
  }

  return (await response.json()) as GitHubRepository[];
}
