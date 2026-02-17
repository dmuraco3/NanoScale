import Link from "next/link";
import { ArrowLeft, GitBranch, ExternalLink, Clock, Server } from "lucide-react";
import { AuthGuard } from "@/components/auth-guard";
import { DashboardLayout } from "@/components/layout";
import { Card, CardHeader, CardTitle, CardContent, Badge, Button } from "@/components/ui";
import { fetchProjectById } from "@/lib/projects-api";

interface ProjectDetailsPageProps {
  params: Promise<{ id: string }>;
}

async function ProjectDetailsPage(props: ProjectDetailsPageProps) {
  const { id } = await props.params;
  const project = await fetchProjectById(id);

  if (!project) {
    return (
      <DashboardLayout>
        <p className="text-sm text-[var(--foreground-muted)]">Project not found.</p>
      </DashboardLayout>
    );
  }

  return (
    <DashboardLayout>
      {/* Back link */}
      <Link
        href="/projects"
        className="inline-flex items-center gap-2 text-sm text-[var(--foreground-secondary)] hover:text-[var(--foreground)] mb-6"
      >
        <ArrowLeft className="h-4 w-4" />
        Back to Projects
      </Link>

      {/* Page header */}
      <div className="flex items-start justify-between mb-8">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-semibold text-[var(--foreground)]">{project.name}</h1>
            <Badge variant="success" dot>
              {project.status}
            </Badge>
          </div>
          <p className="text-[var(--foreground-secondary)] mt-1 flex items-center gap-2">
            <GitBranch className="h-4 w-4" />
            {project.branch}
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="outline">
            View Logs
          </Button>
          <Button>
            Redeploy
          </Button>
        </div>
      </div>

      {/* Info cards grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              Repository
            </CardTitle>
          </CardHeader>
          <CardContent className="mt-0">
            <a
              href={project.repo_url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[var(--accent)] hover:underline flex items-center gap-1"
            >
              {project.repo_url.replace("https://github.com/", "")}
              <ExternalLink className="h-3 w-3" />
            </a>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              Server
            </CardTitle>
          </CardHeader>
          <CardContent className="mt-0">
            <span className="flex items-center gap-2 text-[var(--foreground)]">
              <Server className="h-4 w-4" />
              {project.server_name ?? project.server_id}
            </span>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              Last Deployed
            </CardTitle>
          </CardHeader>
          <CardContent className="mt-0">
            <span className="flex items-center gap-2 text-[var(--foreground)]">
              <Clock className="h-4 w-4" />
              {new Date(project.created_at).toLocaleDateString()}
            </span>
          </CardContent>
        </Card>
      </div>

      <Card className="mb-8">
        <CardHeader>
          <CardTitle>Run Command</CardTitle>
        </CardHeader>
        <CardContent>
          <code className="text-sm text-[var(--foreground-secondary)]">{project.run_command}</code>
        </CardContent>
      </Card>

      {/* Deployment history */}
      <Card>
        <CardHeader>
          <CardTitle>Deployment History</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-[var(--foreground-muted)]">
            No deployment history available yet.
          </p>
        </CardContent>
      </Card>
    </DashboardLayout>
  );
}

export default AuthGuard(ProjectDetailsPage);
