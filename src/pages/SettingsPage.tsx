import { useConfig, useSetActiveWorkspace, useAddWorkspace, useRemoveWorkspace } from "@/hooks/useWorkspace";
import { useQueryClient } from "@tanstack/react-query";
import { cn } from "@/lib/utils";
import { Trash2, Plus, Check, Folder, Terminal } from "lucide-react";
import { toast } from "sonner";
import { tauriApi } from "@/lib/tauri";
import { AiDashboard } from "@/components/ai/AiDashboard";
import { logger } from "@/lib/logger";
import { useState } from "react";
import { Switch } from "@/components/ui/switch";
import { ConfirmDialog, PromptDialog } from "@/components/ui/dialog";



export default function SettingsPage() {
  const queryClient = useQueryClient();
  const { data: config } = useConfig();
  const { mutate: setActive } = useSetActiveWorkspace();
  const { mutateAsync: addWorkspace } = useAddWorkspace();
  const { mutateAsync: removeWorkspace } = useRemoveWorkspace();

  const [workspaceToRemove, setWorkspaceToRemove] = useState<string | null>(null);
  const [showAddWorkspace, setShowAddWorkspace] = useState(false);
  const [showClearLogsConfirm, setShowClearLogsConfirm] = useState(false);

  const handleRemove = async (id: string) => {
    try { 
      await removeWorkspace(id); 
      toast.success("Removed"); 
    } catch { 
      toast.error("Failed to remove"); 
    }
  };

  const handleAdd = async (name: string) => {
    try {
      await addWorkspace({ name: name.trim() });
      toast.success("Workspace created");
    } catch { 
      toast.error("Failed to add workspace"); 
    }
  };

  const handleCleanup = async () => {
    try {
      const count = await tauriApi.cleanupOrphanedBoards();
      if (count > 0) {
        toast.success(`Removed ${count} orphaned board${count !== 1 ? 's' : ''}`);
        queryClient.invalidateQueries({ queryKey: ["boards"] });
      } else {
        toast.info("No orphaned boards found");
      }
    } catch (e) {
      toast.error("Cleanup failed: " + e);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <header className="page-header">
        <h1 className="text-[13px] font-semibold">Settings</h1>
      </header>

      <div className="flex-1 overflow-y-auto">
        <div className="max-w-xl mx-auto px-5 py-6 space-y-8">

          {/* ── Workspaces ── */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <h2 className="settings-section-title">Workspaces</h2>
              <div className="flex items-center gap-3">
                <button
                  onClick={handleCleanup}
                  className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-red-500 transition-colors"
                  title="Remove boards that are no longer in any workspace folder"
                >
                  <Trash2 size={11} /> Cleanup Orphans
                </button>
                <button
                  onClick={() => setShowAddWorkspace(true)}
                  className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
                >
                  <Plus size={11} /> Create New
                </button>
              </div>
            </div>

            <div className="rounded-lg border border-border/60 overflow-hidden bg-card">
              {config?.workspaces.length === 0 && (
                <div className="px-4 py-6 text-center text-[12px] text-muted-foreground/40">No workspaces added</div>
              )}
              {config?.workspaces.map((ws, i) => {
                const isActive = ws.id === config.active;
                const boardCount = ws.board_ids.length;
                return (
                  <div
                    key={ws.id}
                    className={cn(
                      "flex items-center gap-3 px-4 py-3 transition-colors",
                      i > 0 && "border-t border-border/40",
                      isActive ? "bg-secondary/40" : "hover:bg-secondary/20"
                    )}
                  >
                    <Folder size={14} className={cn("shrink-0", isActive ? "text-foreground/60" : "text-muted-foreground/30")} />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-[12.5px] font-medium truncate">{ws.name}</span>
                        {isActive && (
                          <span className="flex items-center gap-0.5 text-[10px] text-emerald-600 dark:text-emerald-400 font-medium shrink-0">
                            <Check size={9} /> Active
                          </span>
                        )}
                      </div>
                      <p className="text-[10.5px] text-muted-foreground/40 truncate font-mono mt-0.5">{boardCount} board{boardCount !== 1 ? 's' : ''}</p>
                    </div>
                    <div className="flex items-center gap-0.5 shrink-0">
                      {!isActive && (
                        <button
                          onClick={() => setActive(ws.id)}
                          className="px-2.5 py-1 text-[11px] font-medium rounded-md bg-secondary hover:bg-border transition-colors"
                        >
                          Switch
                        </button>
                      )}
                      <button
                        onClick={() => setWorkspaceToRemove(ws.id)}
                        className="p-1.5 rounded text-muted-foreground/40 hover:text-red-500 hover:bg-red-500/10 transition-colors"
                        title="Remove"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* ── AI Sort (placeholder) ── */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <h2 className="settings-section-title">AI Sort</h2>
              <span className="text-[10px] bg-secondary px-2 py-0.5 rounded text-muted-foreground/50 font-medium">Coming soon</span>
            </div>
            <div className="rounded-lg border border-border/40 bg-secondary/20 px-4 py-3">
              <p className="text-[12px] text-muted-foreground/50 leading-relaxed">
                Define tags like <em className="not-italic font-medium text-muted-foreground">nature</em>, <em className="not-italic font-medium text-muted-foreground">architecture</em>, <em className="not-italic font-medium text-muted-foreground">animals</em> and PinLocal will automatically generate smart boards using AI semantic search.
              </p>
            </div>
          </section>

          {/* ── AI Search (inline) ── */}
          <AiDashboard />

          {/* Developer & Logging */}
          <section>
            <h2 className="settings-section-title mb-3">Developer & Logging</h2>
            <div className="rounded-lg border border-border/60 overflow-hidden bg-card">
              {/* Row 1: Enable Row */}
              <div className="setting-row px-4">
                <div>
                  <p className="setting-label">Production Logs</p>
                  <p className="setting-desc">Persistent app.log and webview.log files</p>
                </div>
                <Switch
                  checked={config?.logging_enabled}
                  onCheckedChange={(checked) => {
                    tauriApi.setLoggingEnabled(checked)
                      .then(() => {
                        queryClient.invalidateQueries({ queryKey: ["config"] });
                        toast.success(checked ? "Logs enabled" : "Logs disabled");
                      })
                      .catch(() => toast.error("Failed to toggle logs"));
                  }}
                />

              </div>

              {/* Row 2: Management Buttons */}
              <div className="px-4 py-3 flex items-center gap-2">
                <button
                  onClick={() => {
                    tauriApi.openLogsFolder().catch(e => toast.error("Failed to open logs: " + e));
                    logger.info("User requested to open logs folder");
                  }}
                  className="flex items-center gap-2 px-3 py-1.5 text-[11px] font-medium rounded-md bg-secondary hover:bg-border transition-colors"
                >
                  <Terminal size={12} className="text-muted-foreground" />
                  Open Logs Folder
                </button>
                <button
                  onClick={() => setShowClearLogsConfirm(true)}
                  className="px-3 py-1.5 text-[11px] font-medium rounded-md text-red-500 hover:bg-red-500/10 transition-colors"
                >
                  Clear Logs
                </button>
              </div>
            </div>
          </section>

          {/* ── Dialogs ── */}
          <ConfirmDialog
            isOpen={!!workspaceToRemove}
            onClose={() => setWorkspaceToRemove(null)}
            title="Remove Workspace?"
            description="This only removes the configuration from PinLocal. Your physical images and folders will remain safe on your disk."
            confirmText="Remove Workspace"
            variant="destructive"
            onConfirm={() => workspaceToRemove && handleRemove(workspaceToRemove)}
          />

          <PromptDialog
            isOpen={showAddWorkspace}
            onClose={() => setShowAddWorkspace(false)}
            title="New Workspace"
            description="Enter a name for your workspace. This helps you organize different collections of images."
            placeholder="e.g. My Illustrations, Photography..."
            confirmText="Create Workspace"
            onConfirm={handleAdd}
          />

          <ConfirmDialog
            isOpen={showClearLogsConfirm}
            onClose={() => setShowClearLogsConfirm(false)}
            title="Clear All Logs?"
            description="This will permanently delete app.log and webview.log. This action cannot be undone."
            confirmText="Clear Logs"
            variant="destructive"
            onConfirm={async () => {
              try {
                await tauriApi.clearLogs();
                toast.success("Logs cleared");
                logger.info("Logs cleared by user");
              } catch (e) {
                toast.error("Failed to clear logs");
              }
            }}
          />

        </div>
      </div>
    </div>
  );
}
