
import { headers } from "next/headers";

import { clientApiBaseUrl } from "@/lib/api-base-url";

export const dynamic = "force-dynamic";

async function getManifest(appName: string) {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const res = await fetch(
    `${clientApiBaseUrl()}/api/integrations/github/manifest/${encodeURIComponent(appName)}`,
    {
      headers: {
        cookie,
      },
      cache: "no-store",
    },
  );

  if (!res.ok) {
    return "";
  }

  const data = (await res.json()) as { manifest: string };
  return data.manifest;
}

export default async function GitHubSetupPage() {
  const appName = "NanoScale-GitHub-App";
  const manifest = await getManifest(appName);

  return (
    <div className="max-w-xl mx-auto mt-12 p-6 bg-white rounded shadow">
      <h2 className="text-xl font-bold mb-4">Create GitHub App</h2>
      <p className="mt-2 text-sm text-gray-600">
        NanoScale will prefill the GitHub App manifest for this instance and send you back here after GitHub finishes setup.
      </p>
      <form
        action="https://github.com/settings/apps/new"
        method="post"
        className="mt-6"
      >
        <input
          type="hidden"
          name="manifest"
          id="manifest"
          value={manifest}
        />
        <button
          type="submit"
          disabled={manifest.length === 0}
          className="bg-blue-600 text-white px-4 py-2 rounded font-semibold hover:bg-blue-700"
        >
          Continue to GitHub
        </button>
      </form>
      <p className="mt-4 text-sm text-gray-600">
        GitHub will redirect back to NanoScale after the app is created, and NanoScale will save the returned app credentials securely.
      </p>
    </div>
  );
}
