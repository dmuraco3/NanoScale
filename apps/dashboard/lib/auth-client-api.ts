import { clientApiBaseUrl } from "@/lib/api-base-url";

export interface ChangeCredentialsPayload {
  username: string;
  password: string;
}

export async function changeCredentials(payload: ChangeCredentialsPayload): Promise<void> {
  const response = await fetch(`${clientApiBaseUrl()}/api/auth/change-credentials`, {
    method: "POST",
    credentials: "include",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(payload),
  });

  if (response.status === 400) {
    throw new Error("Username is required and password must be at least 8 characters.");
  }

  if (response.status === 401) {
    throw new Error("Your session has expired. Please log in again.");
  }

  if (!response.ok) {
    throw new Error("Unable to update credentials");
  }
}
