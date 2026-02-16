import { AuthGuard } from "@/components/auth-guard";
import { fetchServers } from "@/lib/servers-api";

import ServersPageClient from "./servers-page-client";

async function ServersPage() {
  const initialServers = await fetchServers();

  return <ServersPageClient initialServers={initialServers} />;
}

export default AuthGuard(ServersPage);
