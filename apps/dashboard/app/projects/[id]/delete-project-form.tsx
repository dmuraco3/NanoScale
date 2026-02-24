"use client";

import { useTransition } from "react";
import { Card, CardHeader, CardTitle, CardContent, Button } from "@/components/ui";
import { deleteProjectAction } from "./actions";
import { redirect } from "next/navigation";

interface DeleteProjectFormProps {
  projectName: string;
  projectId: string;
}

export default function DeleteProjectForm({ projectName, projectId }: DeleteProjectFormProps) {
  const [isPending, startTransition] = useTransition();

  function handleSubmit(formData: FormData) {
    const confirmationName = formData.get("confirmationName");
    const typedName = typeof confirmationName === "string" ? confirmationName.trim() : "";

    if (typedName != projectName) {
      redirect(
        `/projects/${projectId}?deleteError=${encodeURIComponent("Project name confirmation does not match")}`,
      );
    }

    startTransition(async () => {
      await deleteProjectAction(projectId);
    });
  }

  return (
    <Card className="mb-8 border-[var(--error)]">
      <CardHeader>
        <CardTitle>Delete Project</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-[var(--foreground-secondary)] mb-4">
          This action cannot be undone. To confirm, type
          <span className="text-[var(--foreground)] font-medium"> {projectName}</span>
          {" "}and click Delete Project.
        </p>
        <form action={handleSubmit} className="flex flex-col sm:flex-row gap-3 sm:items-end">
          <div className="flex-1">
            <label
              htmlFor="confirmation-name"
              className="block text-sm font-medium text-[var(--foreground)] mb-1.5"
            >
              Confirm project name
            </label>
            <input
              id="confirmation-name"
              name="confirmationName"
              type="text"
              required
              placeholder={projectName}
              className="w-full rounded-md border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm text-[var(--foreground)] placeholder:text-[var(--foreground-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:ring-offset-1 focus:ring-offset-[var(--background)]"
            />
          </div>
          <Button type="submit" variant="danger" isLoading={isPending}>
            {isPending ? "Deleting..." : "Delete Project"}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
