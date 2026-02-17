import Link from "next/link";
import { Server, FolderKanban, Activity, ArrowUpRight, Plus } from "lucide-react";
import { AuthGuard } from "@/components/auth-guard";
import { DashboardLayout } from "@/components/layout";
import { Card, CardHeader, CardTitle, CardContent, Button, Badge } from "@/components/ui";
import { fetchServers } from "@/lib/servers-api";
import { fetchProjects } from "@/lib/projects-api";

async function HomePage() {
  const [servers, projects] = await Promise.all([
    fetchServers(),
    fetchProjects(),
  ]);

  const onlineServers = servers.filter((s) => s.status.toLowerCase() === "online");
  const totalRamUsage = servers.length > 0
    ? Math.round(servers.reduce((acc, s) => acc + s.ram_usage_percent, 0) / servers.length)
    : 0;

  return (
    <DashboardLayout>
      {/* Page header */}
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-semibold text-[var(--foreground)]">Dashboard</h1>
          <p className="text-[var(--foreground-secondary)] mt-1">
            Welcome back. Here&apos;s an overview of your infrastructure.
          </p>
        </div>
        <Link href="/projects/new">
          <Button leftIcon={<Plus className="h-4 w-4" />}>
            New Project
          </Button>
        </Link>
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              Total Servers
            </CardTitle>
            <Server className="h-4 w-4 text-[var(--foreground-muted)]" />
          </CardHeader>
          <CardContent className="mt-0">
            <div className="text-2xl font-bold text-[var(--foreground)]">{servers.length}</div>
            <p className="text-xs text-[var(--foreground-muted)] mt-1">
              {onlineServers.length} online, {servers.length - onlineServers.length} offline
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              Active Projects
            </CardTitle>
            <FolderKanban className="h-4 w-4 text-[var(--foreground-muted)]" />
          </CardHeader>
          <CardContent className="mt-0">
            <div className="text-2xl font-bold text-[var(--foreground)]">{projects.length}</div>
            <p className="text-xs text-[var(--foreground-muted)] mt-1">
              Deployed across your infrastructure
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              Avg RAM Usage
            </CardTitle>
            <Activity className="h-4 w-4 text-[var(--foreground-muted)]" />
          </CardHeader>
          <CardContent className="mt-0">
            <div className="text-2xl font-bold text-[var(--foreground)]">{totalRamUsage}%</div>
            <div className="mt-2 h-2 w-full rounded-full bg-[var(--background-tertiary)]">
              <div
                className="h-2 rounded-full bg-[var(--accent)] transition-all"
                style={{ width: `${totalRamUsage}%` }}
              />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Main content grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Projects */}
        <Card padding="none">
          <div className="flex items-center justify-between p-4 border-b border-[var(--border)]">
            <h2 className="font-semibold text-[var(--foreground)]">Recent Projects</h2>
            <Link
              href="/projects"
              className="text-sm text-[var(--accent)] hover:underline flex items-center gap-1"
            >
              View all <ArrowUpRight className="h-3 w-3" />
            </Link>
          </div>
          <div className="divide-y divide-[var(--border)]">
            {projects.length === 0 ? (
              <div className="p-8 text-center">
                <FolderKanban className="h-8 w-8 mx-auto text-[var(--foreground-muted)] mb-3" />
                <p className="text-sm text-[var(--foreground-muted)]">No projects yet</p>
                <Link href="/projects/new">
                  <Button variant="outline" size="sm" className="mt-3">
                    Create your first project
                  </Button>
                </Link>
              </div>
            ) : (
              projects.slice(0, 5).map((project) => (
                <Link
                  key={project.id}
                  href={`/projects/${project.id}`}
                  className="flex items-center justify-between p-4 hover:bg-[var(--background-tertiary)]/50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <div className="h-8 w-8 rounded-md bg-[var(--background-tertiary)] flex items-center justify-center">
                      <FolderKanban className="h-4 w-4 text-[var(--foreground-secondary)]" />
                    </div>
                    <div>
                      <p className="text-sm font-medium text-[var(--foreground)]">{project.name}</p>
                      <p className="text-xs text-[var(--foreground-muted)]">{project.branch}</p>
                    </div>
                  </div>
                  <Badge variant="secondary">{project.status}</Badge>
                </Link>
              ))
            )}
          </div>
        </Card>

        {/* Servers */}
        <Card padding="none">
          <div className="flex items-center justify-between p-4 border-b border-[var(--border)]">
            <h2 className="font-semibold text-[var(--foreground)]">Servers</h2>
            <Link
              href="/servers"
              className="text-sm text-[var(--accent)] hover:underline flex items-center gap-1"
            >
              View all <ArrowUpRight className="h-3 w-3" />
            </Link>
          </div>
          <div className="divide-y divide-[var(--border)]">
            {servers.length === 0 ? (
              <div className="p-8 text-center">
                <Server className="h-8 w-8 mx-auto text-[var(--foreground-muted)] mb-3" />
                <p className="text-sm text-[var(--foreground-muted)]">No servers connected</p>
                <Link href="/servers">
                  <Button variant="outline" size="sm" className="mt-3">
                    Add your first server
                  </Button>
                </Link>
              </div>
            ) : (
              servers.slice(0, 5).map((server) => (
                <div
                  key={server.id}
                  className="flex items-center justify-between p-4"
                >
                  <div className="flex items-center gap-3">
                    <div className="h-8 w-8 rounded-md bg-[var(--background-tertiary)] flex items-center justify-center">
                      <Server className="h-4 w-4 text-[var(--foreground-secondary)]" />
                    </div>
                    <div>
                      <p className="text-sm font-medium text-[var(--foreground)]">{server.name}</p>
                      <p className="text-xs text-[var(--foreground-muted)]">{server.ip_address}</p>
                    </div>
                  </div>
                  <Badge
                    variant={server.status.toLowerCase() === "online" ? "success" : "error"}
                    dot
                  >
                    {server.status.toLowerCase() === "online" ? "Online" : "Offline"}
                  </Badge>
                </div>
              ))
            )}
          </div>
        </Card>
      </div>
    </DashboardLayout>
  );
}

export default AuthGuard(HomePage);
