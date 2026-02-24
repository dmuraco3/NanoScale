"use client";

import { useTransition } from "react";
import { Button } from "@/components/ui";

interface RedeployButtonProps {
  redeployAction: () => Promise<void>;
}

export default function RedeployButton({ redeployAction }: RedeployButtonProps) {
  const [isPending, startTransition] = useTransition();

  function handleClick() {
    startTransition(async () => {
      await redeployAction();
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
