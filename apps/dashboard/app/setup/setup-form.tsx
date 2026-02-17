"use client";

import { useState } from "react";
import { UserPlus } from "lucide-react";

import { setupAdminAction } from "@/app/setup/actions";
import { Button, Input, Card, CardContent } from "@/components/ui";
import { useToast } from "@/components/toast";

export default function SetupForm() {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { addToast } = useToast();

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (password !== confirmPassword) {
      addToast({
        type: "error",
        message: "Passwords don't match",
        description: "Please make sure both passwords are the same.",
      });
      return;
    }

    if (password.length < 8) {
      addToast({
        type: "error",
        message: "Password too short",
        description: "Password must be at least 8 characters long.",
      });
      return;
    }

    setIsSubmitting(true);

    try {
      const response = await setupAdminAction(username, password);

      if (response.ok) {
        addToast({
          type: "success",
          message: "Account created",
          description: "Redirecting to dashboard...",
        });
        window.location.assign("/");
        return;
      }

      let errorMessage = "Unable to create admin account.";
      if (response.status === 400) {
        errorMessage = "Username is required and password must be at least 8 characters.";
      } else if (response.status === 409) {
        errorMessage = "Setup has already been completed. Please sign in.";
      } else if (response.status >= 500) {
        errorMessage = "Server error while creating admin account.";
      } else if (response.status === 0) {
        errorMessage = "Cannot reach NanoScale API. Verify the service is running.";
      }

      addToast({
        type: "error",
        message: "Setup failed",
        description: errorMessage,
      });
    } catch {
      addToast({
        type: "error",
        message: "Setup failed",
        description: "Unable to create admin account.",
      });
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <main className="min-h-screen flex items-center justify-center bg-[var(--background)] px-4">
      <div className="w-full max-w-md">
        {/* Logo and header */}
        <div className="text-center mb-8">
          <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--foreground)] mb-4">
            <span className="text-xl font-bold text-[var(--background)]">N</span>
          </div>
          <h1 className="text-2xl font-semibold text-[var(--foreground)]">Welcome to NanoScale</h1>
          <p className="text-[var(--foreground-secondary)] mt-2">
            Create your admin account to get started
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
                placeholder="Choose a username"
                required
                autoFocus
              />

              <Input
                label="Password"
                id="password"
                type="password"
                value={password}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
                placeholder="Choose a password"
                hint="Must be at least 8 characters"
                minLength={8}
                required
              />

              <Input
                label="Confirm Password"
                id="confirm-password"
                type="password"
                value={confirmPassword}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setConfirmPassword(e.target.value)}
                placeholder="Confirm your password"
                minLength={8}
                required
              />

              <Button
                type="submit"
                disabled={isSubmitting}
                isLoading={isSubmitting}
                className="w-full"
                leftIcon={!isSubmitting ? <UserPlus className="h-4 w-4" /> : undefined}
              >
                Create Admin Account
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
