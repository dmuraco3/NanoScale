import { AuthGuard } from "@/components/auth-guard";
import { fetchServers, type ServerListItem } from "@/lib/servers-api";
import { fetchServerStats, type ServerStatsResponse } from "@/lib/server-stats-api";
import { redirect } from "next/navigation";

import ServerDetailsPageClient from "./server-details-page-client";

interface ServerDetailsPageProps {
  params: { id: string };
}

async function ServerDetailsPage(props: ServerDetailsPageProps) {
  const { id } = props.params;

  const servers = await fetchServers();
  const serverById = servers.find((item) => item.id === id) ?? null;
  const serverByName = serverById ? null : (servers.find((item) => item.name === id) ?? null);

  if (!serverById && serverByName) {
    redirect(`/servers/${serverByName.id}`);
  }

  const server = serverById;

  const initialStats: ServerStatsResponse | null = server ? await fetchServerStats(server.id) : null;

  return (
    <ServerDetailsPageClient
      server={server}
      initialStats={initialStats}
    />
  );
}

export default AuthGuard(ServerDetailsPage);

export type { ServerListItem };
