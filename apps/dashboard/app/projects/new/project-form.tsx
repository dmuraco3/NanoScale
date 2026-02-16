"use client";

import { useState } from "react";

import { type ServerListItem } from "@/lib/servers-api";
import { createProject, type ProjectEnvVar } from "@/lib/projects-api";

interface ProjectFormProps {
  servers: ServerListItem[];
}

export default function ProjectForm(props: ProjectFormProps) {
  const [repoUrl, setRepoUrl] = useState("");
  const [branch, setBranch] = useState("main");
  const [name, setName] = useState("");
  const [buildCommand, setBuildCommand] = useState("bun run build");
  const [serverId, setServerId] = useState(props.servers[0]?.id ?? "");
  const [envVars, setEnvVars] = useState<ProjectEnvVar[]>([{ key: "", value: "" }]);
  const [isSubmitting, setSubmitting] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

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
    setErrorMessage("");

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
        env_vars: filteredEnvVars,
      });

      window.location.assign(`/projects/${result.id}`);
    } catch {
      setSubmitting(false);
      setErrorMessage("Unable to create project.");
    }
  }

  return (
    <main className="min-h-screen bg-zinc-950 p-6 text-zinc-100">
      <h1 className="text-2xl font-semibold">New Project</h1>

      <form className="mt-6 space-y-6" onSubmit={handleSubmit}>
        <section className="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
          <h2 className="text-lg font-medium">Source</h2>
          <div className="mt-4 space-y-3">
            <div>
              <label className="mb-1 block text-sm text-zinc-300" htmlFor="repo-url">
                repo_url
              </label>
              <input
                id="repo-url"
                className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                value={repoUrl}
                onChange={(event) => setRepoUrl(event.target.value)}
                placeholder="https://github.com/owner/repo"
                required
              />
            </div>

            <div>
              <label className="mb-1 block text-sm text-zinc-300" htmlFor="branch">
                branch
              </label>
              <input
                id="branch"
                className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                value={branch}
                onChange={(event) => setBranch(event.target.value)}
                required
              />
            </div>
          </div>
        </section>

        <section className="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
          <h2 className="text-lg font-medium">Configuration</h2>
          <div className="mt-4 space-y-3">
            <div>
              <label className="mb-1 block text-sm text-zinc-300" htmlFor="name">
                name
              </label>
              <input
                id="name"
                className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                value={name}
                onChange={(event) => setName(event.target.value)}
                required
              />
            </div>

            <div>
              <label className="mb-1 block text-sm text-zinc-300" htmlFor="build-command">
                build_command
              </label>
              <input
                id="build-command"
                className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                value={buildCommand}
                onChange={(event) => setBuildCommand(event.target.value)}
                required
              />
            </div>

            <div>
              <label className="mb-1 block text-sm text-zinc-300" htmlFor="server-id">
                server_id
              </label>
              <select
                id="server-id"
                className="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                value={serverId}
                onChange={(event) => setServerId(event.target.value)}
                required
              >
                {props.servers.map((server) => (
                  <option key={server.id} value={server.id}>
                    {server.name}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </section>

        <section className="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
          <h2 className="text-lg font-medium">Environment</h2>
          <div className="mt-4 space-y-2">
            {envVars.map((row, index) => (
              <div key={`${index}-${row.key}`} className="grid grid-cols-12 gap-2">
                <input
                  className="col-span-5 rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                  placeholder="KEY"
                  value={row.key}
                  onChange={(event) =>
                    updateEnvVar(index, { ...row, key: event.target.value })
                  }
                />
                <input
                  className="col-span-5 rounded border border-zinc-700 bg-zinc-950 px-3 py-2"
                  placeholder="VALUE"
                  value={row.value}
                  onChange={(event) =>
                    updateEnvVar(index, { ...row, value: event.target.value })
                  }
                />
                <button
                  type="button"
                  className="col-span-2 rounded border border-zinc-700 px-2 py-2 text-sm"
                  onClick={() => removeEnvVarRow(index)}
                  disabled={envVars.length === 1}
                >
                  Remove
                </button>
              </div>
            ))}
            <button
              type="button"
              className="rounded border border-zinc-700 px-3 py-2 text-sm"
              onClick={addEnvVarRow}
            >
              Add Row
            </button>
          </div>
        </section>

        <button
          type="submit"
          disabled={isSubmitting || props.servers.length === 0}
          className="rounded bg-zinc-100 px-4 py-2 text-zinc-900 disabled:opacity-60"
        >
          {isSubmitting ? "Creating..." : "Create Project"}
        </button>

        {errorMessage ? <p className="text-sm text-red-300">{errorMessage}</p> : null}
      </form>
    </main>
  );
}
