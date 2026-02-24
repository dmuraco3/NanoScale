"use client";

import { useState } from "react";
import { Rocket } from "lucide-react";

import { type ServerListItem } from "@/lib/servers-api";
import { createProject, type ProjectEnvVar } from "@/lib/projects-api";
import { DashboardLayout } from "@/components/layout";
import { Button, Input, Select, Card, CardHeader, CardTitle, CardContent } from "@/components/ui";
import { useToast } from "@/components/toast";
import { EnvVarsCard } from "./env-vars-card";

interface ProjectFormProps {
  servers: ServerListItem[];
}

type AppPresetKey = "nextjs";

interface AppPreset {
  key: AppPresetKey;
  label: string;
  buildCommand: string;
  installCommand: string;
  runCommand: string;
  outputDirectory: string;
}

const APP_PRESETS: AppPreset[] = [
  {
    key: "nextjs",
    label: "Next.js",
    buildCommand: "bun run build",
    installCommand: "bun install --frozen-lockfile",
    runCommand: "bun run start",
    outputDirectory: ".",
  },
];

export default function ProjectForm(props: ProjectFormProps) {
  const defaultPreset = APP_PRESETS[0];
  const { addToast } = useToast();

  const [repoUrl, setRepoUrl] = useState("");
  const [branch, setBranch] = useState("main");
  const [name, setName] = useState("");
  const [preset, setPreset] = useState<AppPresetKey>(defaultPreset.key);
  const [buildCommand, setBuildCommand] = useState(defaultPreset.buildCommand);
  const [installCommand, setInstallCommand] = useState(defaultPreset.installCommand);
  const [runCommand, setRunCommand] = useState(defaultPreset.runCommand);
  const [outputDirectory, setOutputDirectory] = useState(defaultPreset.outputDirectory);
  const [port, setPort] = useState("");
  const [serverId, setServerId] = useState(props.servers[0]?.id ?? "");
  const [envVars, setEnvVars] = useState<ProjectEnvVar[]>([{ key: "", value: "" }]);
  const [isSubmitting, setSubmitting] = useState(false);

  function handlePresetChange(nextPresetKey: AppPresetKey) {
    setPreset(nextPresetKey);

    const selectedPreset = APP_PRESETS.find((item) => item.key === nextPresetKey);
    if (!selectedPreset) {
      return;
    }

    setBuildCommand(selectedPreset.buildCommand);
    setInstallCommand(selectedPreset.installCommand);
    setRunCommand(selectedPreset.runCommand);
    setOutputDirectory(selectedPreset.outputDirectory);
  }

  function updateEnvVar(index: number, next: ProjectEnvVar) {
    setEnvVars((current) => current.map((entry, i) => (i === index ? next : entry)));
  }

  function addEnvVarRow() {
    setEnvVars((current) => [...current, { key: "", value: "" }]);
  }

  function removeEnvVarRow(index: number) {
    setEnvVars((current) => current.filter((_, i) => i !== index));
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);

    try {
      const filteredEnvVars = envVars.filter(
        (item) => item.key.trim().length > 0 || item.value.trim().length > 0,
      );

      const result = await createProject({
        server_id: serverId,
        name,
        repo_url: repoUrl,
        branch,
        build_command: buildCommand,
        install_command: installCommand,
        run_command: runCommand,
        output_directory: outputDirectory,
        port: port.trim().length > 0 ? Number(port) : undefined,
        env_vars: filteredEnvVars,
      });

      if (!result.ok) {
        setSubmitting(false);
        addToast({
          type: "error",
          message: "Failed to create project",
          description: result.message,
        });
        console.error(new Error(result.message, {cause: 'createProject'}))
        return;
      }

      addToast({
        type: "success",
        message: "Project created",
        description: "Redirecting to project page...",
      });
      window.location.assign(`/projects/${result.data.id}`);
    } catch (error) {
      setSubmitting(false);
      console.error(error)
      addToast({
        type: "error",
        message: "Failed to create project",
        description: error instanceof Error ? error.message : "Unable to create project.",
      });
    }
  }

  return (
    <DashboardLayout>
      {/* Page header */}
      <div className="mb-8">
        <h1 className="text-2xl font-semibold text-[var(--foreground)]">New Project</h1>
        <p className="text-[var(--foreground-secondary)] mt-1">
          Deploy a new application from a Git repository.
        </p>
      </div>

      <form className="space-y-6 max-w-3xl" onSubmit={handleSubmit}>
        {/* Source section */}
        <Card>
          <CardHeader>
            <CardTitle>Source</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <Input
                label="Repository URL"
                id="repo-url"
                value={repoUrl}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setRepoUrl(e.target.value)}
                placeholder="https://github.com/owner/repo"
                hint="The Git repository URL to clone"
                required
              />

              <Input
                label="Branch"
                id="branch"
                value={branch}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setBranch(e.target.value)}
                placeholder="main"
                required
              />
            </div>
          </CardContent>
        </Card>

        {/* Configuration section */}
        <Card>
          <CardHeader>
            <CardTitle>Configuration</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <Input
                label="Project Name"
                id="name"
                value={name}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
                placeholder="my-awesome-project"
                required
              />

              <Select
                label="Framework Preset"
                id="app-preset"
                value={preset}
                onChange={(e: React.ChangeEvent<HTMLSelectElement>) => handlePresetChange(e.target.value as AppPresetKey)}
                required
              >
                {APP_PRESETS.map((item) => (
                  <option key={item.key} value={item.key}>
                    {item.label}
                  </option>
                ))}
              </Select>

              <Input
                label="Install Command"
                id="install-command"
                value={installCommand}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setInstallCommand(e.target.value)}
                placeholder="bun install --frozen-lockfile"
                required
              />

              <Input
                label="Build Command"
                id="build-command"
                value={buildCommand}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setBuildCommand(e.target.value)}
                placeholder="bun run build"
                required
              />

              <Input
                label="Run Command"
                id="run-command"
                value={runCommand}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setRunCommand(e.target.value)}
                placeholder="bun run start"
                required
              />

              <Input
                label="Output Directory"
                id="output-directory"
                value={outputDirectory}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setOutputDirectory(e.target.value)}
                placeholder=".next/standalone"
                required
              />

              <Input
                label="Application Port (Optional)"
                id="port"
                type="number"
                min={3100}
                max={65535}
                step={1}
                value={port}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPort(e.target.value)}
                placeholder="Auto-assign"
                hint="Leave empty to auto-assign. Minimum allowed port is 3100."
              />

              <Select
                label="Deploy to Server"
                id="server-id"
                value={serverId}
                onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setServerId(e.target.value)}
                required
              >
                {props.servers.map((server) => (
                  <option key={server.id} value={server.id}>
                    {server.name}
                  </option>
                ))}
              </Select>
            </div>
          </CardContent>
        </Card>

        <EnvVarsCard
          envVars={envVars}
          onAddRow={addEnvVarRow}
          onRemoveRow={removeEnvVarRow}
          onUpdateRow={updateEnvVar}
        />

        {/* Submit */}
        <div className="flex items-center gap-4">
          <Button
            type="submit"
            disabled={isSubmitting || props.servers.length === 0}
            isLoading={isSubmitting}
            leftIcon={!isSubmitting ? <Rocket className="h-4 w-4" /> : undefined}
          >
            Deploy Project
          </Button>
          {props.servers.length === 0 && (
            <p className="text-sm text-[var(--warning)]">
              You need to add a server before creating a project.
            </p>
          )}
        </div>
      </form>
    </DashboardLayout>
  );
}
