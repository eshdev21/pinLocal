import { useEffect, useRef } from "react";
import { useQueryClient, InfiniteData } from "@tanstack/react-query";
import { tauriApi } from "@/lib/tauri";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { useUIStore } from "@/stores/uiStore";
import { logger } from "@/lib/logger";
import { useStatusStore } from "@/stores/statusStore";
import type { PaginatedImages, ThumbnailUpdate } from "@/types";

/**
 * A centralized hook to manage all background synchronization events.
 * Rewritten for the new actor-based workspace system.
 */
export function useWorkspaceSync() {
  const queryClient = useQueryClient();
  const { setActiveBoardId } = useUIStore();
  const unlisteners = useRef<UnlistenFn[]>([]);

  useEffect(() => {
    let mounted = true;

    const setupListeners = async () => {
      let initialTaskIds: string[] = [];

      // 0. Initial Snapshot Reconciliation
      // Fetch active tasks from the backend on startup
      try {
        const config = await tauriApi.getConfig();
        if (config.active_tasks && config.active_tasks.length > 0) {
          initialTaskIds = config.active_tasks.map(t => t.id);
          logger.info(`Found ${config.active_tasks.length} active tasks on startup. Initializing toasts.`);
          config.active_tasks.forEach(task => {
            if (task.status === "running" || task.status === "pending") {
              const progressText = task.total > 0 ? ` (${task.progress}/${task.total})` : "";
              toast.loading(`${task.message || "Working..."}${progressText}`, { id: task.id });
            }
          });
        }
      } catch (err) {
        logger.error("Failed to fetch initial task snapshot", err);
      }

      // 1. Unified State Pulse (SSOT Mirror)
      const uSync = await listen<any>("app:sync", (event) => {
        if (!mounted) return;
        const status = event.payload;

        // Update the centralized status store (New SSOT)
        const previousTasks = useStatusStore.getState().activeTasks;
        useStatusStore.getState().sync(status);

        // Update static caches only if they exist (Legacy support)
        queryClient.setQueryData(["workspaceStatus"], status);

        const currentTasks = status.active_tasks || [];
        const currentTaskIds = new Set(currentTasks.map((t: any) => t.id));

        // 1. DISMISSAL GUARD: If a toast is active but the backend says it's gone, kill it.
        // This is critical for fast production builds where events might be missed.
        previousTasks.forEach(t => {
          if (!currentTaskIds.has(t.id)) {
            logger.info(`Auto-dismissing stale task toast: ${t.id}`);
            toast.dismiss(t.id);
          }
        });

        // 2. UPDATE/CREATE: Show toasts for currently active tasks
        currentTasks.forEach((task: any) => {
          if (task.status === "running" || task.status === "pending") {
            const progressText = task.total > 0 ? ` (${task.progress}/${task.total})` : "";
            toast.loading(`${task.message || "Working..."}${progressText}`, { id: task.id });
          }
        });
      });
      if (mounted) unlisteners.current.push(uSync); else uSync();

      // 2. Workspace Changed: Reset UI
      const u1 = await listen<{ id: string; is_switch: boolean }>("workspace-changed", async (event) => {
        if (!mounted) return;
        const { is_switch } = event.payload;

        logger.info(`Event received: workspace-changed. Resetting state.`);

        if (is_switch) {
          await queryClient.resetQueries({ queryKey: ["images"] });
          await queryClient.resetQueries({ queryKey: ["boards"] });
          await queryClient.resetQueries({ queryKey: ["config"] });
          setActiveBoardId(null);
          toast.info("Switched workspace");
        }
      });
      if (mounted) unlisteners.current.push(u1); else u1();

      // 3. Global Task Finished (Toasts)
      const u4 = await listen<{ id: string; status: string; message?: string }>("app:task-finished", (event) => {
        if (!mounted) return;
        const { id, status, message } = event.payload;

        if (status === "completed") {
          toast.success(message || "Task completed", { id, duration: 2000 });
          // Hard dismissal failsafe
          setTimeout(() => toast.dismiss(id), 2100);
          queryClient.refetchQueries({ queryKey: ["images"] });
          queryClient.refetchQueries({ queryKey: ["boards"] });
        } else if (status === "failed") {
          toast.error(message || "Task failed", { id });
        }
      });
      if (mounted) unlisteners.current.push(u4); else u4();

      // 5. Batch Thumbnails Ready (Surgical patch for performance)
      const u5 = await listen<{ updates: ThumbnailUpdate[] }>("thumbnails:batch-ready", (event) => {
        if (!mounted) return;
        const updates = event.payload?.updates;
        if (!updates?.length) return;

        logger.info(`Event received: thumbnails:batch-ready. Patching cache.`);
        const updateMap = new Map(updates.map(u => [u.id, u]));

        queryClient.setQueriesData<InfiniteData<PaginatedImages>>(
          { queryKey: ["images"] },
          (old) => {
            if (!old) return old;
            return {
              ...old,
              pages: old.pages.map(page => ({
                ...page,
                images: page.images.map(img => {
                  const u = updateMap.get(img.id);
                  if (!u) return img;
                  return {
                    ...img,
                    thumb_path: u.thumb_path,
                    thumbnail_status: 'ready' as const,
                    width: u.width,
                    height: u.height
                  };
                })
              }))
            };
          }
        );
      });
      if (mounted) unlisteners.current.push(u5); else u5();

      // 6. FINAL RECONCILIATION: Check if any initial toasts should be dismissed
      // This closes the race window between the snapshot and the pulse loop starting.
      try {
        const freshStatus = await tauriApi.getAppStatus();
        const stillActive = new Set((freshStatus.active_tasks || []).map((t: any) => t.id));

        initialTaskIds.forEach(id => {
          if (!stillActive.has(id)) {
            logger.info(`Startup Reconciliation: Dismissing finished task toast: ${id}`);
            toast.dismiss(id);
          }
        });
      } catch (err) {
        logger.error("Startup Reconciliation failed", err);
      }
    };

    setupListeners();

    return () => {
      mounted = false;
      unlisteners.current.forEach(u => u());
      unlisteners.current = [];
    };
  }, [queryClient, setActiveBoardId]);
}
