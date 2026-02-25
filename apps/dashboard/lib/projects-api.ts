'use server';

import { headers } from "next/headers";

export interface ProjectEnvVar {
  key: string;
  value: string;
}

export interface ProjectListItem {
  id: string;
  name: string;
  repo_url: string;
  branch: string;
  run_command: string;
  port: number;
  domain: string | null;
  source_provider: string;
  source_repo_id: number | null;
  status: string;
  created_at: string;
}

export interface ProjectDetailsItem {
  id: string;
  server_id: string;
  server_name: string | null;
  name: string;
  repo_url: string;
  branch: string;
  install_command: string;
  build_command: string;
  run_command: string;
  status: string;
  port: number;
  domain: string | null;
  source_provider: string;
  source_repo_id: number | null;
  created_at: string;
}

export interface GitHubProjectSource {
  installation_id: number;
  repo_id: number;
  selected_branch: string;
}

export interface CreateProjectPayload {
  server_id: string;
  name: string;
  repo_url: string;
  branch: string;
  build_command: string;
  install_command: string;
  run_command: string;
  output_directory: string;
  port?: number;
  env_vars: ProjectEnvVar[];
  github_source?: GitHubProjectSource;
}

export interface CreateProjectResponse {
  id: string;
  domain: string | null;
}

export type CreateProjectResult =
  | { ok: true; data: CreateProjectResponse }
  | { ok: false; message: string };

export type DeleteProjectResult =
  | { ok: true }
  | { ok: false; message: string };

export type RedeployProjectResult =
  | { ok: true }
  | { ok: false; message: string };

export async function fetchProjects(): Promise<ProjectListItem[]> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  try {
    const response = await fetch(`${internalApiUrl}/api/projects`, {
      headers: { cookie },
      cache: "no-store",
    });

    if (!response.ok) {
      return [];
    }

    return (await response.json()) as ProjectListItem[];
  } catch {
    return [];
  }
}

export async function fetchProjectById(projectId: string): Promise<ProjectDetailsItem | null> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  try {
    const response = await fetch(`${internalApiUrl}/api/projects/${projectId}`, {
      headers: { cookie },
      cache: "no-store",
    });

    if (!response.ok) {
      console.error(`fetchProjectById\tERROR\t${response.status}\t${response.statusText}`)
      return null;
    }

    return (await response.json()) as ProjectDetailsItem;
  } catch {
    return null;
  }
}

export async function createProject(
  payload: CreateProjectPayload,
): Promise<CreateProjectResult> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  const response = await fetch(`${internalApiUrl}/api/projects`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      cookie,
    },
    body: JSON.stringify(payload),
    cache: "no-store",
  });

  if (!response.ok) {
    let message = `Unable to create project (HTTP ${response.status}).`;
    const rawErrorBody = await response.text();

    try {
      const errorPayload = JSON.parse(rawErrorBody) as { message?: string };
      if (errorPayload.message && errorPayload.message.length > 0) {
        message = errorPayload.message;
      }
    } catch {
      if (rawErrorBody.length > 0) {
        message = rawErrorBody;
      }
    }

    return { ok: false, message };
  }

  return { ok: true, data: (await response.json()) as CreateProjectResponse };
}

export async function deleteProjectById(projectId: string): Promise<DeleteProjectResult> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  const response = await fetch(`${internalApiUrl}/api/projects/${projectId}`, {
    method: "DELETE",
    headers: {
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    let message = `Unable to delete project (HTTP ${response.status}).`;
    const rawErrorBody = await response.text();

    try {
      const errorPayload = JSON.parse(rawErrorBody) as { message?: string };
      if (errorPayload.message && errorPayload.message.length > 0) {
        message = errorPayload.message;
      }
    } catch {
      if (rawErrorBody.length > 0) {
        message = rawErrorBody;
      }
    }

    return { ok: false, message };
  }

  return { ok: true };
}

export async function redeployProjectById(projectId: string): Promise<RedeployProjectResult> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  const response = await fetch(`${internalApiUrl}/api/projects/${projectId}/redeploy`, {
    method: "POST",
    headers: {
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    let message = `Unable to redeploy project (HTTP ${response.status}).`;
    const rawErrorBody = await response.text();

    try {
      const errorPayload = JSON.parse(rawErrorBody) as { message?: string };
      if (errorPayload.message && errorPayload.message.length > 0) {
        message = errorPayload.message;
      }
    } catch {
      if (rawErrorBody.length > 0) {
        message = rawErrorBody;
      }
    }

    return { ok: false, message };
  }

  return { ok: true };
}
