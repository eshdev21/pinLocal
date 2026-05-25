import { useBoards } from "@/hooks/useBoards";
import { useConfig, useRenameWorkspace, useAddFoldersToWorkspace, useRemoveBoardFromWorkspace } from "@/hooks/useWorkspace";
import { useUIStore } from "@/stores/uiStore";
import { cn } from "@/lib/utils";
import { Plus, Home, Settings, Folder, Sun, Moon, Sparkles, Edit2, Edit3, Check, Trash2 } from "lucide-react";
import { Link, useLocation } from "react-router-dom";
import { toast } from "sonner";
import { AiStatusDots } from "../ai/AiStatusDots";
import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { PromptDialog, ConfirmDialog } from "../ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator, DropdownMenuLabel } from "../ui/dropdown-menu";
import { MoreHorizontal } from "lucide-react";



export default function Sidebar() {
  const location = useLocation();
  const { data: config } = useConfig();
  const { data: boards } = useBoards();
  const { activeBoardId, setActiveBoardId, theme, toggleTheme } = useUIStore();

  const renameWorkspace = useRenameWorkspace();
  const addFolders = useAddFoldersToWorkspace();
  const removeBoard = useRemoveBoardFromWorkspace();

  // Initialize global event listeners

  const [showRename, setShowRename] = useState(false);
  const [showFolderRemove, setShowFolderRemove] = useState<number | null>(null);
  const [manageMode, setManageMode] = useState(false);

  const activeWorkspace = config?.workspaces.find(w => w.id === config.active);

  const handleRenameWorkspace = (name: string) => {
    if (!activeWorkspace) return;
    if (name.trim() && name !== activeWorkspace.name) {
      renameWorkspace.mutate({ id: activeWorkspace.id, name: name.trim() });
    }
  };

  const handleAddFolders = async () => {
    const selected = await open({
      directory: true,
      multiple: true,
      title: "Select Folders to Add to Workspace"
    });

    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      
      // Immediate feedback
      const toastId = toast.loading("Adding folders and scanning images...");
      
      addFolders.mutate(paths, {
        onSuccess: () => {
          toast.success(`${paths.length} folder(s) added. Indexing in progress...`, { id: toastId });
        },
        onError: (err: any) => {
          toast.error(`Failed to add folders: ${err.toString()}`, { id: toastId });
        }
      });
    }
  };

  const handleRemoveFolder = (boardId: number) => {
    removeBoard.mutate(boardId, {
      onSuccess: () => {
        toast.success("Folder removed from workspace");
        setShowFolderRemove(null);
      },
      onError: (err: any) => toast.error(err.toString())
    });
  };

  const isSettings = location.pathname === "/settings";
  const isHome = !isSettings && activeBoardId === null;

  return (
    <aside
      className="flex flex-col h-screen shrink-0 border-r overflow-hidden"
      style={{ width: 220, background: "hsl(var(--sidebar))", borderColor: "hsl(var(--sidebar-border))" }}
    >
      {/* Workspace Header */}
      <div className="flex items-center justify-between px-4 h-11 border-b shrink-0" style={{ borderColor: "hsl(var(--sidebar-border))" }}>
        <div className="flex items-center gap-2 min-w-0 group cursor-pointer" onClick={() => setShowRename(true)}>
          <span className="text-[13px] font-semibold text-foreground tracking-tight truncate">
            {activeWorkspace?.name || "PinLocal"}
          </span>
          <Edit2 size={10} className="text-muted-foreground/0 group-hover:text-muted-foreground/40 transition-colors shrink-0" />
        </div>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={toggleTheme}
              className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-secondary/60 transition-colors"
            >
              {theme === "dark" ? <Sun size={13} /> : <Moon size={13} />}
            </button>
          </TooltipTrigger>
          <TooltipContent>{theme === "dark" ? "Switch to light" : "Switch to dark"}</TooltipContent>
        </Tooltip>
      </div>

      {/* Nav */}
      <nav className="flex-1 overflow-y-auto px-2 py-2 space-y-0.5">
        <Link to="/" onClick={() => setActiveBoardId(null)}
          className={cn("nav-item", isHome && "nav-item-active")}
        >
          <Home size={13} className="shrink-0" />
          <span>All Images</span>
        </Link>

        <Link to="/settings"
          className={cn("nav-item", isSettings && "nav-item-active")}
        >
          <Settings size={13} className="shrink-0" />
          <span>Settings</span>
        </Link>

        <button
          className="nav-item w-full text-left opacity-50 cursor-default"
          onClick={() => toast.info("AI Sort is coming soon")}
        >
          <Sparkles size={13} className="shrink-0" />
          <span>AI Sort</span>
          <span className="ml-auto text-[10px] bg-secondary px-1.5 py-0.5 rounded font-medium text-muted-foreground">Soon</span>
        </button>

        {/* Boards / Folders section */}
        <div className="pt-5 pb-1 px-2.5">
          <div className="flex items-center justify-between h-6">
            <span className="text-[10px] font-bold uppercase tracking-[0.12em] text-muted-foreground/30">
              Folders
            </span>
            <div className="flex items-center gap-0.5">
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => setManageMode(!manageMode)}
                    className={cn(
                      "p-1 rounded-md transition-all",
                      manageMode
                        ? "bg-primary/20 text-primary hover:bg-primary/30"
                        : "text-muted-foreground/60 hover:text-foreground hover:bg-secondary/80"
                    )}
                  >
                    {manageMode ? <Check size={11} /> : <Edit3 size={11} />}
                  </button>
                </TooltipTrigger>
                <TooltipContent>{manageMode ? "Finish managing" : "Manage folders"}</TooltipContent>
              </Tooltip>

              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={handleAddFolders}
                    className="p-1 rounded-md text-muted-foreground/60 hover:text-foreground hover:bg-secondary/80 transition-all"
                  >
                    <Plus size={11} />
                  </button>
                </TooltipTrigger>
                <TooltipContent>Add Source Folder</TooltipContent>
              </Tooltip>
            </div>
          </div>
        </div>

        <div className="space-y-0.5">
          {boards?.map((board) => {
            const isActive = !isSettings && activeBoardId === board.id;
            return (
              <Link
                key={board.id}
                to="/"
                onClick={() => setActiveBoardId(board.id)}
                className={cn("nav-item justify-between group/item", isActive && "nav-item-active")}
              >
                <div className="flex items-center gap-2 min-w-0 flex-1">
                  <Folder
                    size={12}
                    className={cn("shrink-0 transition-colors", isActive ? "text-foreground" : "text-muted-foreground/40")}
                  />
                  <span className="truncate">{board.name}</span>
                </div>

                <div className="flex items-center gap-1.5 shrink-0 pr-1">
                  {!manageMode ? (
                    <span className="text-[10px] tabular-nums text-muted-foreground/20 group-hover/item:text-muted-foreground/40 transition-colors">
                      {board.image_count}
                    </span>
                  ) : (
                    <DropdownMenu>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <DropdownMenuTrigger asChild>
                            <button className="p-1 rounded-md text-muted-foreground/60 hover:text-foreground hover:bg-secondary/80 transition-all">
                              <MoreHorizontal size={12} />
                            </button>
                          </DropdownMenuTrigger>
                        </TooltipTrigger>
                        <TooltipContent>Folder Actions</TooltipContent>
                      </Tooltip>
                      <DropdownMenuContent align="end" className="w-40">
                        <DropdownMenuLabel>Folder Actions</DropdownMenuLabel>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem 
                          className="text-destructive focus:text-destructive"
                          onClick={() => setShowFolderRemove(board.id)}
                        >
                          <Trash2 size={12} className="mr-2" />
                          Remove from Workspace
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  )}
                </div>
              </Link>
            );
          })}
        </div>
      </nav>

      <div className="mt-auto px-3 py-3">
        <button
          onClick={handleAddFolders}
          className="flex items-center justify-center gap-2 w-full px-3 py-2 text-[12px] font-bold rounded-lg bg-foreground text-background hover:bg-foreground/90 transition-all shadow-sm active:scale-[0.98] group"
        >
          <Plus size={14} className="group-hover:rotate-90 transition-transform duration-200" />
          <span>Add Folder</span>
        </button>
      </div>

      {/* Footer status */}
      <div className="px-3 py-2.5 border-t space-y-1.5 shrink-0" style={{ borderColor: "hsl(var(--sidebar-border))" }}>
        <div className="flex items-center gap-2">
          <span className="status-dot bg-emerald-500" />
          <span className="text-[10px] text-muted-foreground/50 font-medium tracking-wide">Watching Folders</span>
        </div>
        <AiStatusDots showLabel={true} className="opacity-80" />
      </div>

      <PromptDialog
        isOpen={showRename}
        onClose={() => setShowRename(false)}
        title="Rename Workspace"
        description="Choose a new name for your workspace."
        defaultValue={activeWorkspace?.name}
        placeholder="e.g. My Illustrations..."
        confirmText="Rename"
        onConfirm={handleRenameWorkspace}
      />


      <ConfirmDialog
        isOpen={!!showFolderRemove}
        onClose={() => setShowFolderRemove(null)}
        title="Remove Folder"
        description="Are you sure you want to remove this folder from the workspace? All its images will be unlinked."
        confirmText="Remove"
        variant="destructive"
        onConfirm={() => showFolderRemove && handleRemoveFolder(showFolderRemove)}
      />
    </aside>
  );
}
