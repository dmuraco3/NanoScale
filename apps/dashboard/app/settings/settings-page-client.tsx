"use client";

import { useState } from "react";

import { DashboardLayout } from "@/components/layout";
import { useToast } from "@/components/toast";
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Input } from "@/components/ui";
import {
  disconnectGitHubIntegration,
  fetchGitHubStatus,
  startGitHubIntegration,
  type GitHubStatus,
} from "@/lib/github-api";
import { changeCredentials } from "@/lib/auth-client-api";
import {
  fetchUpdateStatus,
  pollHealthUntilReady,
  triggerUpdate,
  type UpdateStatus,
} from "@/lib/update-api";

interface SettingsPageClientProps {
  initialGitHubStatus: GitHubStatus | null;
  initialUpdateStatus: UpdateStatus | null;
}

export function SettingsPageClient({
  initialGitHubStatus,
  initialUpdateStatus,
}: SettingsPageClientProps) {
  const { addToast } = useToast();

  const [githubStatus, setGitHubStatus] = useState<GitHubStatus | null>(initialGitHubStatus);
  const [isRefreshingGitHubStatus, setRefreshingGitHubStatus] = useState(false);
  const [isDisconnectingGitHub, setDisconnectingGitHub] = useState(false);

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [isChangingCredentials, setChangingCredentials] = useState(false);
  const [isUpdating, setUpdating] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(initialUpdateStatus);
  const [isCheckingForUpdates, setCheckingForUpdates] = useState(false);

  async function handleRefreshGitHubStatus() {
    setRefreshingGitHubStatus(true);

    try {
      const status = await fetchGitHubStatus();
      setGitHubStatus(status);

      if (!status) {
        addToast({
          type: "error",
          message: "Unable to load GitHub status",
        });
        return;
      }

      addToast({
        type: "success",
        message: "GitHub status updated",
      });
    } catch (error) {
      addToast({
        type: "error",
        message: "Unable to load GitHub status",
        description: error instanceof Error ? error.message : "Try again.",
      });
    } finally {
      setRefreshingGitHubStatus(false);
    }
  }

  async function handleIntegrateGitHub() {
    try {
      const redirectUrl = await startGitHubIntegration();
      window.location.assign(redirectUrl);
    } catch (error) {
      addToast({
        type: "error",
        message: "Unable to start GitHub integration",
        description: error instanceof Error ? error.message : "Try again.",
      });
    }
  }

  async function handleDisconnectGitHub() {
    setDisconnectingGitHub(true);

    try {
      await disconnectGitHubIntegration();
      addToast({
        type: "success",
        message: "GitHub disconnected",
      });

      const status = await fetchGitHubStatus();
      setGitHubStatus(status);
    } catch (error) {
      addToast({
        type: "error",
        message: "Unable to disconnect GitHub",
        description: error instanceof Error ? error.message : "Try again.",
      });
    } finally {
      setDisconnectingGitHub(false);
    }
  }

  async function handleChangeCredentials(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (password !== confirmPassword) {
      addToast({
        type: "error",
        message: "Passwords do not match",
      });
      return;
    }

    setChangingCredentials(true);

    try {
      await changeCredentials({ username, password });
      setPassword("");
      setConfirmPassword("");
      addToast({
        type: "success",
        message: "Credentials updated",
      });
    } catch (error) {
      addToast({
        type: "error",
        message: "Unable to update credentials",
        description: error instanceof Error ? error.message : "Try again.",
      });
    } finally {
      setChangingCredentials(false);
    }
  }

  async function handleUpdateNanoScale() {
    if (!updateStatus?.update_available) {
      return;
    }

    setUpdating(true);

    try {
      await triggerUpdate();
      await pollHealthUntilReady(2000);
      window.location.reload();
    } catch (error) {
      addToast({
        type: "error",
        message: "Unable to update NanoScale",
        description: error instanceof Error ? error.message : "Try again.",
      });
      setUpdating(false);
    }
  }

  async function handleCheckForUpdates() {
    setCheckingForUpdates(true);

    try {
      const status = await fetchUpdateStatus();
      setUpdateStatus(status);

      addToast({
        type: "success",
        message: status.update_available
          ? `Update available: ${status.latest_version}`
          : "You are already on the latest version",
      });
    } catch (error) {
      addToast({
        type: "error",
        message: "Unable to check for updates",
        description: error instanceof Error ? error.message : "Try again.",
      });
    } finally {
      setCheckingForUpdates(false);
    }
  }

  return (
    <DashboardLayout>
      <div className="mb-8">
        <h1 className="text-2xl font-semibold text-[var(--foreground)]">Settings</h1>
        <p className="mt-1 text-[var(--foreground-secondary)]">
          Manage account security and GitHub integration.
        </p>
      </div>

      <div className="space-y-6 max-w-3xl">
        <Card>
          <CardHeader>
            <CardTitle>GitHub Integration</CardTitle>
            <CardDescription>
              Configure the GitHub App NanoScale uses to read repositories and receive webhooks.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-[var(--foreground-secondary)]">
              {githubStatus === null
                ? "Status unknown. Click Refresh GitHub Status."
                : githubStatus.connected
                  ? `GitHub App configured: ${githubStatus.github_login ?? "NanoScale GitHub App"}.`
                  : "GitHub App is not configured yet."}
            </p>

            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="secondary"
                onClick={handleRefreshGitHubStatus}
                isLoading={isRefreshingGitHubStatus}
              >
                Refresh GitHub Status
              </Button>
              <Button type="button" onClick={handleIntegrateGitHub}>
                Set Up GitHub App
              </Button>
              {githubStatus?.connected && (
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleDisconnectGitHub}
                  isLoading={isDisconnectingGitHub}
                >
                  Disconnect GitHub App
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Credentials</CardTitle>
            <CardDescription>
              Update your username and password.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form className="space-y-4" onSubmit={handleChangeCredentials}>
              <Input
                label="Username"
                value={username}
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                  setUsername(event.target.value);
                }}
                autoComplete="username"
                required
              />
              <Input
                label="New Password"
                type="password"
                value={password}
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                  setPassword(event.target.value);
                }}
                autoComplete="new-password"
                required
              />
              <Input
                label="Confirm New Password"
                type="password"
                value={confirmPassword}
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                  setConfirmPassword(event.target.value);
                }}
                autoComplete="new-password"
                required
              />
              <Button type="submit" isLoading={isChangingCredentials}>
                Save Credentials
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Updates</CardTitle>
            <CardDescription>
              Install the latest NanoScale release.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-[var(--foreground-secondary)]">
              Current version: {updateStatus?.current_version ?? "unknown"}
            </p>
            <p className="text-sm text-[var(--foreground-secondary)]">
              Latest version: {updateStatus?.latest_version ?? "unknown"}
            </p>
            <p className="text-sm text-[var(--foreground-secondary)]">
              {updateStatus === null
                ? "Update status unknown. Click Check for updates."
                : updateStatus.update_available
                  ? "A newer release is available."
                  : "You are on the latest release."}
            </p>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="secondary"
                onClick={handleCheckForUpdates}
                isLoading={isCheckingForUpdates}
              >
                Check for updates
              </Button>
              <Button
                type="button"
                onClick={handleUpdateNanoScale}
                isLoading={isUpdating}
                disabled={!updateStatus?.update_available}
              >
                {isUpdating ? "Restarting..." : "Update"}
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </DashboardLayout>
  );
}
