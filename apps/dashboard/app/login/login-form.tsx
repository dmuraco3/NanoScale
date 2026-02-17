"use client";

import { useState } from "react";
import { LogIn } from "lucide-react";

import { clientApiBaseUrl } from "@/lib/api-base-url";
import { Button, Input, Card, CardContent } from "@/components/ui";
import { useToast } from "@/components/toast";

export default function LoginForm() {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { addToast } = useToast();

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSubmitting(true);

    const response = await fetch(`${clientApiBaseUrl()}/api/auth/login`, {
      method: "POST",
      credentials: "include",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({ username, password }),
    });

    if (response.ok) {
      window.location.assign("/");
      return;
    }

    addToast({
      type: "error",
      message: "Invalid credentials",
      description: "Please check your username and password and try again.",
    });
    setIsSubmitting(false);
  }

  return (
    <main className="min-h-screen flex items-center justify-center bg-[var(--background)] px-4">
      <div className="w-full max-w-md">
        {/* Logo and header */}
        <div className="text-center mb-8">
          <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--foreground)] mb-4">
            <span className="text-xl font-bold text-[var(--background)]">N</span>
          </div>
          <h1 className="text-2xl font-semibold text-[var(--foreground)]">Welcome back</h1>
          <p className="text-[var(--foreground-secondary)] mt-2">
            Sign in to your NanoScale dashboard
          </p>
        </div>

        <Card>
          <CardContent className="mt-0 pt-6">
            <form className="space-y-4" onSubmit={handleSubmit}>
              <Input
                label="Username"
                id="username"
                value={username}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUsername(e.target.value)}
                placeholder="Enter your username"
                required
                autoFocus
              />

              <Input
                label="Password"
                id="password"
                type="password"
                value={password}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
                placeholder="Enter your password"
                required
              />

              <Button
                type="submit"
                disabled={isSubmitting}
                isLoading={isSubmitting}
                className="w-full"
                leftIcon={!isSubmitting ? <LogIn className="h-4 w-4" /> : undefined}
              >
                Sign In
              </Button>
            </form>
          </CardContent>
        </Card>

        <p className="text-center text-sm text-[var(--foreground-muted)] mt-6">
          NanoScale Control Plane
        </p>
      </div>
    </main>
  );
}
