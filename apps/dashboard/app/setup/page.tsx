import { redirect } from "next/navigation";

import SetupForm from "./setup-form";
import { fetchAuthStatus } from "@/lib/auth-api";

export default async function SetupPage() {
  const authStatus = await fetchAuthStatus();

  if (authStatus.users_count > 0 && authStatus.authenticated) {
    redirect("/");
  }

  if (authStatus.users_count > 0) {
    redirect("/login");
  }

  return <SetupForm />;
}
