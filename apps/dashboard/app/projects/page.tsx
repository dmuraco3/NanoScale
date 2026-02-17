import Link from "next/link";
import { FolderKanban, Plus, GitBranch, ExternalLink } from "lucide-react";
import { AuthGuard } from "@/components/auth-guard";
import { DashboardLayout } from "@/components/layout";
import { Card, Button, Badge } from "@/components/ui";
import { fetchProjects } from "@/lib/projects-api";

async function ProjectsPage() {
  const projects = await fetchProjects();

  return (
    <DashboardLayout>
      {/* Page header */}
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-semibold text-[var(--foreground)]">Projects</h1>
          <p className="text-[var(--foreground-secondary)] mt-1">
            Manage your deployed applications.
          </p>
        </div>
        <Link href="/projects/new">
          <Button leftIcon={<Plus className="h-4 w-4" />}>
            New Project
          </Button>
        </Link>
      </div>

      {/* Projects grid */}
      {projects.length === 0 ? (
        <Card className="p-12 text-center">
          <FolderKanban className="h-12 w-12 mx-auto text-[var(--foreground-muted)] mb-4" />
          <h2 className="text-lg font-semibold text-[var(--foreground)] mb-2">No projects yet</h2>
          <p className="text-[var(--foreground-secondary)] mb-6 max-w-md mx-auto">
            Get started by creating your first project. Deploy applications from Git repositories to your servers.
          </p>
          <Link href="/projects/new">
            <Button leftIcon={<Plus className="h-4 w-4" />}>
              Create your first project
            </Button>
          </Link>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {projects.map((project) => (
            <Link key={project.id} href={`/projects/${project.id}`}>
              <Card hover className="h-full">
                <div className="flex items-start justify-between mb-4">
                  <div className="h-10 w-10 rounded-lg bg-[var(--background-tertiary)] flex items-center justify-center">
                    <FolderKanban className="h-5 w-5 text-[var(--foreground-secondary)]" />
                  </div>
                  <Badge
                    variant={project.status === "deployed" ? "success" : "secondary"}
                    dot
                  >
                    {project.status}
                  </Badge>
                </div>

                <h3 className="font-semibold text-[var(--foreground)] mb-1">{project.name}</h3>
                
                <div className="flex items-center gap-2 text-sm text-[var(--foreground-muted)]">
                  <GitBranch className="h-3.5 w-3.5" />
                  {project.branch}
                </div>

                <div className="mt-4 pt-4 border-t border-[var(--border)] flex items-center justify-between">
                  <a
                    href={project.repo_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs text-[var(--foreground-muted)] hover:text-[var(--accent)] flex items-center gap-1"
                    onClick={(e) => e.stopPropagation()}
                  >
                    {project.repo_url.replace("https://github.com/", "").slice(0, 25)}
                    {project.repo_url.replace("https://github.com/", "").length > 25 && "..."}
                    <ExternalLink className="h-3 w-3" />
                  </a>
                  <span className="text-xs text-[var(--foreground-muted)]">
                    {new Date(project.created_at).toLocaleDateString()}
                  </span>
                </div>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </DashboardLayout>
  );
}

export default AuthGuard(ProjectsPage);
