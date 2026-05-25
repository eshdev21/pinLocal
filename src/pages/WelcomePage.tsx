import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useSetActiveWorkspace, useConfig, useAddWorkspace } from "@/hooks/useWorkspace";
import { toast } from "sonner";
import { PromptDialog } from "@/components/ui/dialog";

export default function WelcomePage() {
  const [loading, setLoading] = useState(false);
  const { mutate: setActive } = useSetActiveWorkspace();
  const { data: config } = useConfig();
  const { mutateAsync: addWorkspace } = useAddWorkspace();
  const [showAdd, setShowAdd] = useState(false);

  const handleCreateWorkspace = async (name: string) => {
    try {
      setLoading(true);
      await addWorkspace({ name: name.trim() });
      toast.success("Workspace created!");
    } catch (error) {
      toast.error("Failed to create workspace");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen items-center justify-center bg-background text-foreground p-8 animate-in fade-in duration-700">
      <div className="mb-12 text-center">
        <h1 className="text-4xl font-light mb-4 tracking-tight text-foreground/90">PinLocal</h1>
        <p className="text-muted-foreground/70 text-sm max-w-[300px] mx-auto leading-relaxed font-light">
          Your offline image universe. 
          Portable boards for local collections.
        </p>
      </div>

      <div className="flex flex-col gap-3 w-full max-w-xs">
        <Button 
          size="lg" 
          className="h-12 text-sm font-medium rounded-xl" 
          onClick={() => setShowAdd(true)}
          disabled={loading}
        >
          {loading ? "Initializing..." : "Create Workspace"}
        </Button>
      </div>

      <PromptDialog
        isOpen={showAdd}
        onClose={() => setShowAdd(false)}
        title="Welcome to PinLocal"
        description="Let's start by naming your first workspace. You can add image folders to it later."
        placeholder="e.g. My Inspiration, Project Alpha..."
        confirmText="Create Workspace"
        onConfirm={handleCreateWorkspace}
      />

      {config?.workspaces && config.workspaces.length > 0 && (
        <div className="mt-16 w-full max-w-xs">
          <h3 className="text-[10px] font-semibold text-muted-foreground/40 uppercase tracking-[0.2em] mb-4 text-center">Recents</h3>
          <div className="space-y-2">
            {config.workspaces.map((ws) => {
              const boardCount = ws.board_ids.length;
              return (
                <button
                  key={ws.id}
                  onClick={() => setActive(ws.id)}
                  className="w-full flex items-center justify-between p-3 px-4 rounded-xl border border-border/30 bg-card/40 hover:bg-secondary/30 transition-all duration-300 group"
                >
                  <div className="text-left min-w-0">
                    <p className="text-xs font-medium text-foreground/80 truncate">{ws.name}</p>
                    <p className="text-[10px] text-muted-foreground/50 truncate font-mono mt-0.5">{boardCount} board{boardCount !== 1 ? 's' : ''}</p>
                  </div>
                  <div className="text-muted-foreground/30 group-hover:text-foreground/50 transition-colors pl-2">
                    <div className="h-4 w-4 flex items-center justify-center">→</div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
