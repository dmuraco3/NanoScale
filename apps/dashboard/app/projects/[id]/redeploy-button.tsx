"use client";

import { useTransition } from "react";
import { Button } from "@/components/ui";
import { redeployProjectAction } from "./actions";

interface RedeployButtonProps {
  projectId: string
}

export default function RedeployButton({ projectId }: RedeployButtonProps) {
  const [isPending, startTransition] = useTransition();

  function handleClick() {
    startTransition(async () => {
      await redeployProjectAction(projectId);
    });
  }

  return (
    <form action={handleClick}>
      <Button type="submit" isLoading={isPending}>
        {isPending ? "Redeploying..." : "Redeploy"}
      </Button>
    </form>
  );
}
