import { redirect } from "next/navigation";
import type { JSX } from "react";

import { fetchAuthStatus } from "@/lib/auth-api";

type GuardedComponent<Props> = (props: Props) => Promise<JSX.Element> | JSX.Element;

export function AuthGuard<Props>(WrappedComponent: GuardedComponent<Props>) {
  return async function Guarded(props: Props): Promise<JSX.Element> {
    const authStatus = await fetchAuthStatus();

    if (authStatus.users_count === 0) {
      redirect("/setup");
    }

    if (!authStatus.authenticated) {
      redirect("/login");
    }

    return await WrappedComponent(props);
  };
}
