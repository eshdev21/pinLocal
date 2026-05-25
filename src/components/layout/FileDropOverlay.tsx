import { motion, AnimatePresence } from "framer-motion";
import { Folder, Plus } from "lucide-react";
import { useConfig } from "@/hooks/useWorkspace";

interface FileDropOverlayProps {
  isVisible: boolean;
}

/**
 * A minimal, elegant overlay that appears when the user drags folders over the app.
 */
export function FileDropOverlay({ isVisible }: FileDropOverlayProps) {
  const { data: config } = useConfig();
  const activeWorkspace = config?.workspaces.find(w => w.id === config.active);

  return (
    <AnimatePresence>
      {isVisible && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-[100] pointer-events-none flex items-center justify-center"
        >
          {/* Subtle background dim */}
          <div className="absolute inset-0 bg-background/40 backdrop-blur-[4px]" />
          
          {/* Edge border frame */}
          <div className="absolute inset-4 border border-foreground/10 rounded-[var(--radius)] border-dashed" />

          {/* Minimal Centered Pill */}
          <motion.div
            initial={{ scale: 0.98, opacity: 0, y: 5 }}
            animate={{ scale: 1, opacity: 1, y: 0 }}
            exit={{ scale: 0.98, opacity: 0, y: 5 }}
            transition={{ duration: 0.15 }}
            className="relative px-5 py-3 rounded-xl bg-background/95 border border-border shadow-2xl flex items-center gap-3"
          >
            <div className="relative">
              <Folder size={16} className="text-muted-foreground/60" strokeWidth={2} />
              <Plus size={8} className="absolute -top-0.5 -right-0.5 text-foreground" strokeWidth={3} />
            </div>
            <span className="text-[12.5px] font-medium tracking-tight">
              Drop to add folders to <span className="text-foreground font-bold">{activeWorkspace?.name || "Workspace"}</span>
            </span>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
