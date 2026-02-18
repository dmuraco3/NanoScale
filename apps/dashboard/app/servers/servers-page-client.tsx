"use client";

import Link from "next/link";
import { useRef, useState } from "react";
import { Server, Plus, MoreHorizontal, Copy, Check, Loader2 } from "lucide-react";

import {
  type ServerListItem,
  fetchServersClient,
  generateJoinToken,
} from "@/lib/servers-api";
import { DashboardLayout } from "@/components/layout";
import {
  Button,
  Badge,
  Modal,
  ModalFooter,
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
  TableEmpty,
  Dropdown,
  DropdownItem,
} from "@/components/ui";

interface ServersPageClientProps {
  initialServers: ServerListItem[];
}

export default function ServersPageClient(props: ServersPageClientProps) {
  const [servers, setServers] = useState<ServerListItem[]>(props.initialServers);
  const [isModalOpen, setModalOpen] = useState(false);
  const [joinToken, setJoinToken] = useState("");
  const [isGeneratingToken, setGeneratingToken] = useState(false);
  const [isPolling, setPolling] = useState(false);
  const [copyLabel, setCopyLabel] = useState<"copy" | "copied">("copy");
  const pollingActiveRef = useRef(false);

  const orchestratorUrl =
    process.env.NEXT_PUBLIC_NANOSCALE_ORCHESTRATOR_URL ??
    process.env.NEXT_PUBLIC_NANOSCALE_API_URL ??
    "http://localhost:4000";

  async function openModal() {
    setJoinToken("");
    setCopyLabel("copy");
    setModalOpen(true);
  }

  function closeModal() {
    pollingActiveRef.current = false;
    setPolling(false);
    setModalOpen(false);
    setJoinToken("");
    setGeneratingToken(false);
    setCopyLabel("copy");
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
    setCopyLabel("copied");
    setTimeout(() => setCopyLabel("copy"), 2000);
  }

  return (
    <DashboardLayout>
      {/* Page header */}
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-semibold text-[var(--foreground)]">Servers</h1>
          <p className="text-[var(--foreground-secondary)] mt-1">
            Manage your infrastructure and connected servers.
          </p>
        </div>
        <Button leftIcon={<Plus className="h-4 w-4" />} onClick={openModal}>
          Add Server
        </Button>
      </div>

      {/* Servers table */}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>IP Address</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>RAM Usage</TableHead>
            <TableHead className="text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {servers.length === 0 ? (
            <TableEmpty
              colSpan={5}
              message="No servers connected yet"
              icon={<Server className="h-8 w-8" />}
            />
          ) : (
            servers.map((server) => (
              <TableRow key={server.id}>
                <TableCell>
                  <div className="flex items-center gap-3">
                    <div className="h-8 w-8 rounded-md bg-[var(--background-tertiary)] flex items-center justify-center">
                      <Server className="h-4 w-4 text-[var(--foreground-secondary)]" />
                    </div>
                    <Link
                      href={`/servers/${server.id}`}
                      className="font-medium hover:text-[var(--accent)] transition-colors"
                    >
                      {server.name}
                    </Link>
                  </div>
                </TableCell>
                <TableCell className="text-[var(--foreground-secondary)]">
                  {server.ip_address}
                </TableCell>
                <TableCell>
                  <Badge
                    variant={server.status.toLowerCase() === "online" ? "success" : "error"}
                    dot
                  >
                    {server.status.toLowerCase() === "online" ? "Online" : "Offline"}
                  </Badge>
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-3">
                    <div className="h-2 w-24 rounded-full bg-[var(--background-tertiary)]">
                      <div
                        className="h-2 rounded-full bg-[var(--accent)] transition-all"
                        style={{ width: `${server.ram_usage_percent}%` }}
                      />
                    </div>
                    <span className="text-sm text-[var(--foreground-secondary)]">
                      {server.ram_usage_percent}%
                    </span>
                  </div>
                </TableCell>
                <TableCell className="text-right">
                  <Dropdown
                    trigger={
                      <button className="p-2 rounded-md hover:bg-[var(--background-tertiary)] transition-colors">
                        <MoreHorizontal className="h-4 w-4 text-[var(--foreground-secondary)]" />
                      </button>
                    }
                  >
                    <DropdownItem>Edit</DropdownItem>
                    <DropdownItem destructive>Delete</DropdownItem>
                  </Dropdown>
                </TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>

      {/* Add Server Modal */}
      <Modal
        isOpen={isModalOpen}
        onClose={closeModal}
        title="Add Server"
        description="Connect a new server to your NanoScale cluster"
        size="lg"
      >
        <div className="space-y-6">
          {/* Step 1 */}
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <div className="flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-xs font-medium text-white">
                1
              </div>
              <span className="text-sm font-medium text-[var(--foreground)]">Generate Join Token</span>
            </div>
            <Button
              variant="secondary"
              onClick={handleGenerateToken}
              disabled={isGeneratingToken || isPolling}
              isLoading={isGeneratingToken && !joinToken}
            >
              {joinToken ? "Token Generated" : "Generate Token"}
            </Button>
          </div>

          {/* Step 2 */}
          {joinToken && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <div className="flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-xs font-medium text-white">
                  2
                </div>
                <span className="text-sm font-medium text-[var(--foreground)]">
                  Run this command on your server
                </span>
              </div>
              <div className="relative">
                <div className="rounded-lg border border-[var(--border)] bg-[var(--background)] p-4 font-mono text-sm overflow-x-auto">
                  <code className="text-[var(--foreground-secondary)] break-all">
                    curl -sL nanoscale.sh | bash -s -- --join {joinToken} --orchestrator {orchestratorUrl}
                  </code>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="absolute top-2 right-2"
                  onClick={handleCopyCommand}
                >
                  {copyLabel === "copied" ? (
                    <Check className="h-4 w-4 text-[var(--success)]" />
                  ) : (
                    <Copy className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>
          )}

          {/* Step 3 */}
          {isPolling && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <div className="flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-xs font-medium text-white">
                  3
                </div>
                <span className="text-sm font-medium text-[var(--foreground)]">
                  Waiting for connection
                </span>
              </div>
              <div className="flex items-center gap-3 text-sm text-[var(--foreground-secondary)]">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>Listening for new server connections...</span>
              </div>
            </div>
          )}
        </div>

        <ModalFooter>
          <Button variant="outline" onClick={closeModal}>
            Cancel
          </Button>
        </ModalFooter>
      </Modal>
    </DashboardLayout>
  );
}
