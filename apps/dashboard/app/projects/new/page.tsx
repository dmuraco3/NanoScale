import { redirect } from "next/navigation";

import { AuthGuard } from "@/components/auth-guard";
import { fetchServers } from "@/lib/servers-api";

import ProjectForm from "./project-form";

async function NewProjectPage() {
  const servers = await fetchServers();

  if (servers.length === 0) {
    redirect("/servers");
  }

  return <ProjectForm servers={servers} />;
}

export default AuthGuard(NewProjectPage);
