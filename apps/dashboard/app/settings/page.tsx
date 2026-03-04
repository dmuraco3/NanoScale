import { AuthGuard } from "@/components/auth-guard";
import { fetchGitHubStatusServer } from "@/lib/github-server-api";
import { fetchUpdateStatusServer } from "@/lib/update-server-api";
import { SettingsPageClient } from "./settings-page-client";

async function SettingsPage() {
  const initialGitHubStatus = await fetchGitHubStatusServer();
  const initialUpdateStatus = await fetchUpdateStatusServer();

  return (
    <SettingsPageClient
      initialGitHubStatus={initialGitHubStatus}
      initialUpdateStatus={initialUpdateStatus}
    />
  );
}

export default AuthGuard(SettingsPage);
