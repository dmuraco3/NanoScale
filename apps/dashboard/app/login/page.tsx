import { redirect } from "next/navigation";

import LoginForm from "./login-form";
import { fetchAuthStatus } from "@/lib/auth-api";

export default async function LoginPage() {
  const authStatus = await fetchAuthStatus();

  if (authStatus.users_count === 0) {
    redirect("/setup");
  }

  if (authStatus.authenticated) {
    redirect("/");
  }

  return <LoginForm />;
}
