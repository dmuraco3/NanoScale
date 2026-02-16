import { AuthGuard } from "@/components/auth-guard";

interface ProjectDetailsPageProps {
  params: Promise<{ id: string }>;
}

async function ProjectDetailsPage(props: ProjectDetailsPageProps) {
  const { id } = await props.params;

  return (
    <main className="min-h-screen bg-zinc-950 p-6 text-zinc-100">
      <h1 className="text-2xl font-semibold">Project Details</h1>
      <p className="mt-2 text-zinc-300">Project ID: {id}</p>
    </main>
  );
}

export default AuthGuard(ProjectDetailsPage);
