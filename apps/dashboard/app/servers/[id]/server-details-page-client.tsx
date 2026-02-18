"use client";

import useSWR from "swr";
import Link from "next/link";
import { useMemo, useState } from "react";
import { Server, ArrowLeft, Loader2 } from "lucide-react";

import { DashboardLayout } from "@/components/layout";
import { Card, CardHeader, CardTitle, CardDescription, Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableEmpty, Select, Badge } from "@/components/ui";
import type { ServerListItem } from "@/lib/servers-api";
import type { ServerStatsResponse } from "@/lib/server-stats-api";
import { fetchServerStatsClient } from "@/lib/server-stats-client";

interface RefreshOption {
  label: string;
  valueMs: number;
}

const REFRESH_OPTIONS: RefreshOption[] = [
  { label: "2s", valueMs: 2000 },
  { label: "5s", valueMs: 5000 },
  { label: "10s", valueMs: 10000 },
  { label: "30s", valueMs: 30000 },
];

interface ServerDetailsPageClientProps {
  server: ServerListItem | null;
  initialStats: ServerStatsResponse | null;
}

function formatPercent(value: number, digits: number = 1): string {
  if (!Number.isFinite(value)) {
    return "0%";
  }
  return `${value.toFixed(digits)}%`;
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  let unitIndex = 0;
  let value = bytes;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const digits = unitIndex === 0 ? 0 : unitIndex === 1 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unitIndex]}`;
}

function formatBytesPerSec(bytesPerSec: number): string {
  return `${formatBytes(bytesPerSec)}/s`;
}

export default function ServerDetailsPageClient(props: ServerDetailsPageClientProps) {
  const [refreshMs, setRefreshMs] = useState<number>(10000);

  const refreshOptions = useMemo(() => REFRESH_OPTIONS, []);

  const statsKey = props.server ? ["server-stats", props.server.id, refreshMs] : null;

  const { data, isLoading, error } = useSWR(
    statsKey,
    async () => {
      if (!props.server) {
        throw new Error("Server not found");
      }
      return fetchServerStatsClient(props.server.id);
    },
    {
      refreshInterval: refreshMs,
      fallbackData: props.initialStats ?? undefined,
      revalidateOnFocus: false,
    },
  );

  const server = props.server;

  return (
    <DashboardLayout>
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-start gap-3">
          <Link
            href="/servers"
            className="mt-1 inline-flex items-center gap-2 text-sm text-[var(--foreground-muted)] hover:text-[var(--foreground)]"
          >
            <ArrowLeft className="h-4 w-4" />
            Back
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-semibold text-[var(--foreground)]">
                {server ? server.name : "Server"}
              </h1>
              {server && (
                <Badge variant={server.status.toLowerCase() === "online" ? "success" : "error"} dot>
                  {server.status.toLowerCase() === "online" ? "Online" : "Offline"}
                </Badge>
              )}
            </div>
            <p className="mt-1 text-[var(--foreground-secondary)]">
              {server ? server.ip_address : "Unknown server"}
            </p>
          </div>
        </div>

        <div className="w-44">
          <Select
            label="Refresh"
            value={String(refreshMs)}
            onChange={(event) => setRefreshMs(Number(event.target.value))}
            disabled={!server}
          >
            {refreshOptions.map((option) => (
              <option key={option.valueMs} value={String(option.valueMs)}>
                {option.label}
              </option>
            ))}
          </Select>
        </div>
      </div>

      {!server ? (
        <Card className="p-12 text-center">
          <Server className="h-12 w-12 mx-auto text-[var(--foreground-muted)] mb-4" />
          <h2 className="text-lg font-semibold text-[var(--foreground)] mb-2">Server not found</h2>
          <p className="text-[var(--foreground-secondary)]">This server may have been removed.</p>
        </Card>
      ) : (
        <>
          {error && (
            <Card className="mb-6" padding="lg">
              <CardHeader>
                <div>
                  <CardTitle>Unable to load stats</CardTitle>
                  <CardDescription>{String(error)}</CardDescription>
                </div>
              </CardHeader>
            </Card>
          )}

          {/* Totals */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <Card>
              <CardHeader>
                <div>
                  <CardTitle>CPU</CardTitle>
                  <CardDescription>Total utilization</CardDescription>
                </div>
                {isLoading && <Loader2 className="h-4 w-4 animate-spin text-[var(--foreground-muted)]" />}
              </CardHeader>
              <div className="mt-4 text-2xl font-semibold text-[var(--foreground)]">
                {data ? formatPercent(data.totals.cpu_usage_percent) : "—"}
              </div>
              <div className="mt-1 text-sm text-[var(--foreground-muted)]">
                {data ? `${data.totals.cpu_cores} cores` : ""}
              </div>
            </Card>

            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Memory</CardTitle>
                  <CardDescription>Total used</CardDescription>
                </div>
              </CardHeader>
              {data ? (
                <>
                  <div className="mt-4 text-2xl font-semibold text-[var(--foreground)]">
                    {formatBytes(data.totals.used_memory_bytes)}
                  </div>
                  <div className="mt-1 text-sm text-[var(--foreground-muted)]">
                    {formatBytes(data.totals.total_memory_bytes)} total ({formatPercent(
                      (data.totals.used_memory_bytes / Math.max(1, data.totals.total_memory_bytes)) * 100,
                      0,
                    )})
                  </div>
                </>
              ) : (
                <div className="mt-4 text-2xl font-semibold text-[var(--foreground)]">—</div>
              )}
            </Card>

            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Disk</CardTitle>
                  <CardDescription>Total used</CardDescription>
                </div>
              </CardHeader>
              {data ? (
                <>
                  <div className="mt-4 text-2xl font-semibold text-[var(--foreground)]">
                    {formatBytes(data.totals.used_disk_bytes)}
                  </div>
                  <div className="mt-1 text-sm text-[var(--foreground-muted)]">
                    {formatBytes(data.totals.total_disk_bytes)} total ({formatPercent(
                      (data.totals.used_disk_bytes / Math.max(1, data.totals.total_disk_bytes)) * 100,
                      0,
                    )})
                  </div>
                </>
              ) : (
                <div className="mt-4 text-2xl font-semibold text-[var(--foreground)]">—</div>
              )}
            </Card>

            <Card>
              <CardHeader>
                <div>
                  <CardTitle>Network</CardTitle>
                  <CardDescription>Throughput</CardDescription>
                </div>
              </CardHeader>
              {data ? (
                <>
                  <div className="mt-4 text-sm text-[var(--foreground)]">
                    <div className="flex items-center justify-between">
                      <span className="text-[var(--foreground-muted)]">In</span>
                      <span className="font-medium">{formatBytesPerSec(data.totals.network_rx_bytes_per_sec)}</span>
                    </div>
                    <div className="flex items-center justify-between mt-1">
                      <span className="text-[var(--foreground-muted)]">Out</span>
                      <span className="font-medium">{formatBytesPerSec(data.totals.network_tx_bytes_per_sec)}</span>
                    </div>
                  </div>
                  <div className="mt-2 text-xs text-[var(--foreground-muted)]">
                    Totals: {formatBytes(data.totals.network_rx_bytes_total)} in / {formatBytes(
                      data.totals.network_tx_bytes_total,
                    )} out
                  </div>
                </>
              ) : (
                <div className="mt-4 text-2xl font-semibold text-[var(--foreground)]">—</div>
              )}
            </Card>
          </div>

          {/* Per-project breakdown */}
          <div className="mb-3 flex items-end justify-between">
            <div>
              <h2 className="text-lg font-semibold text-[var(--foreground)]">Projects</h2>
              <p className="text-sm text-[var(--foreground-secondary)]">
                CPU, memory, disk, and network usage per project on this server.
              </p>
            </div>
          </div>

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Project</TableHead>
                <TableHead>CPU</TableHead>
                <TableHead>Memory</TableHead>
                <TableHead>Disk</TableHead>
                <TableHead>Network In</TableHead>
                <TableHead>Network Out</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {!data || data.projects.length === 0 ? (
                <TableEmpty
                  colSpan={6}
                  message={data ? "No projects hosted on this server" : "Loading stats…"}
                  icon={isLoading ? <Loader2 className="h-8 w-8 animate-spin" /> : undefined}
                />
              ) : (
                data.projects.map((project) => (
                  <TableRow key={project.project_id}>
                    <TableCell>
                      <div className="font-medium">{project.project_name}</div>
                      <div className="text-xs text-[var(--foreground-muted)]">{project.project_id}</div>
                    </TableCell>
                    <TableCell>{formatPercent(project.cpu_usage_percent)}</TableCell>
                    <TableCell>{formatBytes(project.memory_current_bytes)}</TableCell>
                    <TableCell>{formatBytes(project.disk_usage_bytes)}</TableCell>
                    <TableCell>
                      <div className="text-sm">{formatBytesPerSec(project.network_ingress_bytes_per_sec)}</div>
                      <div className="text-xs text-[var(--foreground-muted)]">{formatBytes(project.network_ingress_bytes_total)} total</div>
                    </TableCell>
                    <TableCell>
                      <div className="text-sm">{formatBytesPerSec(project.network_egress_bytes_per_sec)}</div>
                      <div className="text-xs text-[var(--foreground-muted)]">{formatBytes(project.network_egress_bytes_total)} total</div>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </>
      )}
    </DashboardLayout>
  );
}
