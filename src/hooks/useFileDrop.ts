import { useEffect, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useAddFoldersToWorkspace } from "./useWorkspace";
import { toast } from "sonner";
import { logger } from "@/lib/logger";

interface DragDropPayload {
  paths: string[];
  position: { x: number; y: number };
}

/**
 * Hook to handle OS-level drag and drop events from Tauri.
 */
export function useFileDrop() {
  const [isDragging, setIsDragging] = useState(false);
  const { mutate: addFolders } = useAddFoldersToWorkspace();

  useEffect(() => {
    let mounted = true;
    const unlisteners: UnlistenFn[] = [];

    const setup = async () => {
      const enter = await listen("tauri://drag-enter", () => {
        if (!mounted) return;
        setIsDragging(true);
        logger.info("Drag Enter detected");
      });
      if (mounted) unlisteners.push(enter); else enter();

      const leave = await listen("tauri://drag-leave", () => {
        if (!mounted) return;
        setIsDragging(false);
        logger.info("Drag Leave detected");
      });
      if (mounted) unlisteners.push(leave); else leave();

      const drop = await listen<DragDropPayload>("tauri://drag-drop", (event) => {
        if (!mounted) return;
        setIsDragging(false);
        const paths = event.payload.paths;
        logger.info(`Drop detected: ${paths.length} items`);
        
        if (paths.length > 0) {
          addFolders(paths, {
            onSuccess: () => {},
            onError: (err: any) => {
              toast.error(err.toString());
            }
          });
        }
      });
      if (mounted) unlisteners.push(drop); else drop();
    };

    setup();

    return () => {
      mounted = false;
      unlisteners.forEach(un => un());
    };
  }, [addFolders]);

  return { isDragging };
}
