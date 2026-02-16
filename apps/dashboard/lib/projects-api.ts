import { clientApiBaseUrl } from "@/lib/auth-api";

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
    throw new Error("Unable to create project");
  }

  return (await response.json()) as CreateProjectResponse;
}
