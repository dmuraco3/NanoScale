"use client";

import { useRef, useState } from "react";

import {
  type ServerListItem,
  fetchServersClient,
  generateJoinToken,
} from "@/lib/servers-api";

interface ServersPageClientProps {
  initialServers: ServerListItem[];
}

export default function ServersPageClient(props: ServersPageClientProps) {
  const [servers, setServers] = useState<ServerListItem[]>(props.initialServers);
  const [isModalOpen, setModalOpen] = useState(false);
  const [joinToken, setJoinToken] = useState("");
  const [isGeneratingToken, setGeneratingToken] = useState(false);
  const [isPolling, setPolling] = useState(false);
  const [copyLabel, setCopyLabel] = useState("Copy");
  const pollingActiveRef = useRef(false);

  const orchestratorUrl =
    process.env.NEXT_PUBLIC_NANOSCALE_ORCHESTRATOR_URL ??
    process.env.NEXT_PUBLIC_NANOSCALE_API_URL ??
    "http://localhost:4000";

  async function openModal() {
    setJoinToken("");
    setCopyLabel("Copy");
    setModalOpen(true);
  }

  function closeModal() {
    pollingActiveRef.current = false;
    setPolling(false);
    setModalOpen(false);
    setJoinToken("");
    setGeneratingToken(false);
    setCopyLabel("Copy");
  }

  async function handleGenerateToken() {
    setGeneratingToken(true);

    try {
      const knownServerIds = new Set(servers.map((server) => server.id));
      const token = await generateJoinToken();
      setJoinToken(token);

      pollingActiveRef.current = true;
      setPolling(true);
      void pollForNewServer(knownServerIds);
    } catch {
      setGeneratingToken(false);
      setPolling(false);
    }
  }

  async function pollForNewServer(knownServerIds: Set<string>) {
    while (pollingActiveRef.current) {
      const nextServers = await fetchServersClient();
      const joinedServer = nextServers.find(
        (server) => !knownServerIds.has(server.id) && server.status.toLowerCase() === "online",
      );

      if (joinedServer) {
        setServers(nextServers);
        closeModal();
        return;
      }

      await new Promise((resolve) => {
        setTimeout(resolve, 2000);
      });
    }
  }

  async function handleCopyCommand() {
    if (!joinToken) {
      return;
    }

    const command = `curl -sL nanoscale.sh | bash -s -- --join ${joinToken} --orchestrator ${orchestratorUrl}`;
    await navigator.clipboard.writeText(command);
    setCopyLabel("Copied");
  }

  return (
    <main className="min-h-screen bg-zinc-950 p-6 text-zinc-100">
      <section className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Servers</h1>
        <button
          type="button"
          className="rounded bg-zinc-100 px-3 py-2 text-sm text-zinc-900"
          onClick={openModal}
        >
          Add Server
        </button>
      </section>

      <section className="mt-6 overflow-hidden rounded-lg border border-zinc-800">
        <table className="min-w-full divide-y divide-zinc-800">
          <thead className="bg-zinc-900">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-400">Name</th>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-400">IP Address</th>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-400">Status</th>
              <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-400">RAM Usage</th>
              <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-400">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-zinc-800 bg-zinc-950">
            {servers.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-4 py-6 text-center text-sm text-zinc-400">
                  No servers connected yet.
                </td>
              </tr>
            ) : (
              servers.map((server) => (
                <tr key={server.id}>
                  <td className="px-4 py-3 text-sm text-zinc-200">{server.name}</td>
                  <td className="px-4 py-3 text-sm text-zinc-300">{server.ip_address}</td>
                  <td className="px-4 py-3 text-sm">
                    <span
                      className={
                        server.status.toLowerCase() === "online"
                          ? "text-emerald-400"
                          : "text-red-400"
                      }
                    >
                      {server.status.toLowerCase() === "online" ? "Online" : "Offline"}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-sm text-zinc-300">
                    <div className="h-2 w-full rounded bg-zinc-800">
                      <div
                        className="h-2 rounded bg-zinc-300"
                        style={{ width: `${server.ram_usage_percent}%` }}
                      />
                    </div>
                  </td>
                  <td className="px-4 py-3 text-right text-sm text-zinc-300">
                    <details className="inline-block text-left">
                      <summary className="cursor-pointer list-none rounded px-2 py-1 text-zinc-200">⋯</summary>
                      <div className="mt-1 w-24 rounded border border-zinc-700 bg-zinc-900 p-1">
                        <button type="button" className="block w-full rounded px-2 py-1 text-left text-xs hover:bg-zinc-800">
                          Edit
                        </button>
                        <button type="button" className="block w-full rounded px-2 py-1 text-left text-xs hover:bg-zinc-800">
                          Delete
                        </button>
                      </div>
                    </details>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </section>

      {isModalOpen ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4">
          <section className="w-full max-w-2xl rounded-lg border border-zinc-700 bg-zinc-900 p-6">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-zinc-100">Add Server</h2>
              <button type="button" className="text-zinc-300" onClick={closeModal}>
                Close
              </button>
            </div>

            <div className="mt-4 space-y-4">
              <div>
                <p className="text-sm text-zinc-300">Step 1</p>
                <button
                  type="button"
                  className="mt-2 rounded bg-zinc-100 px-3 py-2 text-sm text-zinc-900 disabled:opacity-70"
                  onClick={handleGenerateToken}
                  disabled={isGeneratingToken || isPolling}
                >
                  {isGeneratingToken ? "Generating..." : "Generate Join Token"}
                </button>
              </div>

              {joinToken ? (
                <div>
                  <p className="text-sm text-zinc-300">Step 2</p>
                  <div className="mt-2 rounded border border-zinc-700 bg-zinc-950 p-3">
                    <code className="block break-all text-xs text-zinc-200">
                      {`curl -sL nanoscale.sh | bash -s -- --join ${joinToken} --orchestrator ${orchestratorUrl}`}
                    </code>
                  </div>
                  <button
                    type="button"
                    className="mt-2 rounded border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
                    onClick={handleCopyCommand}
                  >
                    {copyLabel}
                  </button>
                </div>
              ) : null}

              {isPolling ? (
                <div>
                  <p className="text-sm text-zinc-300">Step 3</p>
                  <p className="mt-2 text-sm text-zinc-200">Waiting for connection...</p>
                </div>
              ) : null}
            </div>
          </section>
        </div>
      ) : null}
    </main>
  );
}
