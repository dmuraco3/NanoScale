import { Plus, Trash2 } from "lucide-react";

import { Button, Input, Card, CardHeader, CardTitle, CardContent } from "@/components/ui";
import { type ProjectEnvVar } from "@/lib/projects-api";

export interface EnvVarsCardProps {
  envVars: ProjectEnvVar[];
  onAddRow: () => void;
  onRemoveRow: (index: number) => void;
  onUpdateRow: (index: number, next: ProjectEnvVar) => void;
}

export function EnvVarsCard(props: EnvVarsCardProps) {
  const { envVars, onAddRow, onRemoveRow, onUpdateRow } = props;

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <CardTitle>Environment Variables</CardTitle>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onAddRow}
          leftIcon={<Plus className="h-4 w-4" />}
        >
          Add Variable
        </Button>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {envVars.map((row, index) => (
            <div key={`${index}-${row.key}`} className="flex gap-3">
              <Input
                className="flex-1"
                placeholder="KEY"
                value={row.key}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  onUpdateRow(index, { ...row, key: e.target.value })
                }
              />
              <Input
                className="flex-1"
                placeholder="VALUE"
                value={row.value}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  onUpdateRow(index, { ...row, value: e.target.value })
                }
              />
              <Button
                type="button"
                variant="ghost"
                size="md"
                onClick={() => onRemoveRow(index)}
                disabled={envVars.length === 1}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
          {envVars.length === 0 && (
            <p className="text-sm text-[var(--foreground-muted)] text-center py-4">
              No environment variables configured
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
