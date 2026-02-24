import Link from "next/link";
import { redirect } from "next/navigation";
import { ArrowLeft, GitBranch, ExternalLink, Clock, Server, Network } from "lucide-react";
import { AuthGuard } from "@/components/auth-guard";
import { DashboardLayout } from "@/components/layout";
import { Card, CardHeader, CardTitle, CardContent, Badge, Button } from "@/components/ui";
import { deleteProjectById, fetchProjectById, redeployProjectById } from "@/lib/projects-api";
import RedeployButton from "./redeploy-button";
import DeleteProjectForm from "./delete-project-form";

interface ProjectDetailsPageProps {
  params: Promise<{ id: string }>;
  searchParams?: Promise<{ deleteError?: string; redeployError?: string }>;
}

async function ProjectDetailsPage(props: ProjectDetailsPageProps) {
  const { id } = await props.params;
  const searchParams = props.searchParams ? await props.searchParams : undefined;
  const project = await fetchProjectById(id);

  const projectUrl = project?.domain
    ? project.domain.startsWith("http://") || project.domain.startsWith("https://")
      ? project.domain
      : `https://${project.domain}`
    : null;

  async function deleteProjectAction(formData: FormData) {
    "use server";

    const confirmationName = formData.get("confirmationName");
    const typedName = typeof confirmationName === "string" ? confirmationName.trim() : "";

    if (typedName != project?.name) {
      redirect(
        `/projects/${id}?deleteError=${encodeURIComponent("Project name confirmation does not match")}`,
      );
    }

    const deleteResult = await deleteProjectById(id);
    if (!deleteResult.ok) {
      redirect(`/projects/${id}?deleteError=${encodeURIComponent(deleteResult.message)}`);
    }

    redirect("/projects");
  }

  async function redeployProjectAction() {
    "use server";

    const redeployResult = await redeployProjectById(id);
    if (!redeployResult.ok) {
      redirect(`/projects/${id}?redeployError=${encodeURIComponent(redeployResult.message)}`);
    }

    redirect(`/projects/${id}`);
  }

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
          <RedeployButton redeployAction={redeployProjectAction} />
        </div>
      </div>

      {searchParams?.deleteError && (
        <Card className="mb-6 border-[var(--error)]">
          <CardContent>
            <p className="text-sm text-[var(--error)]">{searchParams.deleteError}</p>
          </CardContent>
        </Card>
      )}

      {searchParams?.redeployError && (
        <Card className="mb-6 border-[var(--error)]">
          <CardContent>
            <p className="text-sm text-[var(--error)]">{searchParams.redeployError}</p>
          </CardContent>
        </Card>
      )}

      {/* Info cards grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
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

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-[var(--foreground-secondary)]">
              URL
            </CardTitle>
          </CardHeader>
          <CardContent className="mt-0">
            <span className="flex items-center gap-2 text-[var(--foreground)]">
              <Network className="h-4 w-4" />
              {projectUrl ? (
                <a
                  href={projectUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[var(--accent)] hover:underline"
                >
                  {project.domain}
                </a>
              ) : (
                <span className="text-[var(--foreground-secondary)]">Not assigned</span>
              )}
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

      <DeleteProjectForm projectName={project.name} deleteAction={deleteProjectAction} />

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
