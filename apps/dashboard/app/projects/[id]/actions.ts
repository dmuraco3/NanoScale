'use server';

import { deleteProjectById, redeployProjectById } from "@/lib/projects-api";
import { redirect } from "next/navigation";

export async function deleteProjectAction(projectId: string) {
  await new Promise(resolve => setTimeout(resolve, 2500))
  const deleteResult = await deleteProjectById(projectId);
  if (!deleteResult.ok) {
    redirect(`/projects/${projectId}?deleteError=${encodeURIComponent(deleteResult.message)}`);
  }

  redirect("/projects");
}

export async function redeployProjectAction(projectId: string) {
  await new Promise(resolve => setTimeout(resolve, 2500))
  const redeployResult = await redeployProjectById(projectId);
  if (!redeployResult.ok) {
    redirect(`/projects/${projectId}?redeployError=${encodeURIComponent(redeployResult.message)}`);
  }

  redirect(`/projects/${projectId}`);
}