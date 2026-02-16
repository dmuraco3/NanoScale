'use server';

import { clientApiBaseUrl } from "@/lib/api-base-url";

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
  env_vars: ProjectEnvVar[];
}

export interface CreateProjectResponse {
  id: string;
}

interface ApiErrorResponse {
  message?: string;
}

export async function createProject(
  payload: CreateProjectPayload,
): Promise<CreateProjectResponse> {
  const response = await fetch(`${clientApiBaseUrl()}/api/projects`, {
    method: "POST",
    credentials: "include",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    let message = `Unable to create project (HTTP ${response.status}).`;
    const rawErrorBody = await response.text();

    try {
      const errorPayload = JSON.parse(rawErrorBody) as ApiErrorResponse;
      if (errorPayload.message && errorPayload.message.length > 0) {
        message = errorPayload.message;
      }
    } catch {
      if (rawErrorBody.length > 0) {
        message = rawErrorBody;
      }
    }

    throw new Error(message);
  }

  return (await response.json()) as CreateProjectResponse;
}
