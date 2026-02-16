'use server';

import { headers } from "next/headers";

export interface ProjectEnvVar {
  key: string;
  value: string;
}

export interface CreateProjectPayload {
  server_id: string;
  name: string;
  repo_url: string;
  branch: string;
  build_command: string;
  install_command: string;
  output_directory: string;
  env_vars: ProjectEnvVar[];
}

export interface CreateProjectResponse {
  id: string;
}

export type CreateProjectResult =
  | { ok: true; data: CreateProjectResponse }
  | { ok: false; message: string };

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
