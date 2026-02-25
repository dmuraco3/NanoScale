import { AuthGuard } from "@/components/auth-guard";
import { fetchGitHubStatusServer } from "@/lib/github-server-api";
import { SettingsPageClient } from "./settings-page-client";

async function SettingsPage() {
  const initialGitHubStatus = await fetchGitHubStatusServer();

  return <SettingsPageClient initialGitHubStatus={initialGitHubStatus} />;
}

export default AuthGuard(SettingsPage);
