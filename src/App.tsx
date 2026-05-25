import { Routes, Route, Navigate } from "react-router-dom";
import HomePage from "@/pages/HomePage";
import BoardPage from "@/pages/BoardPage";
import WelcomePage from "@/pages/WelcomePage";
import SettingsPage from "@/pages/SettingsPage";
import { useConfig } from "@/hooks/useWorkspace";
import Sidebar from "@/components/layout/Sidebar";
import Lightbox from "@/components/images/Lightbox";
import { Toaster } from "@/components/ui/sonner";
import { useUIStore } from "@/stores/uiStore";
import { useEffect } from "react";
import { useWorkspaceSync } from "@/hooks/useWorkspaceSync";
import { CheckCircle2, AlertCircle, Info, Loader2 } from "lucide-react";
import { logger } from "@/lib/logger";
import { useFileDrop } from "@/hooks/useFileDrop";
import { FileDropOverlay } from "@/components/layout/FileDropOverlay";
import { TooltipProvider } from "@/components/ui/tooltip";

function MainContent() {
  const { activeBoardId } = useUIStore();

  return (
    <div className="flex h-screen w-screen bg-background text-foreground overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-y-auto overflow-x-hidden min-w-0">
        <Routes>
          <Route path="/" element={activeBoardId !== null ? <BoardPage /> : <HomePage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
      <Lightbox />
    </div>
  );
}


function App() {
  const { data: config, isLoading } = useConfig();
  const theme = useUIStore((s) => s.theme);
  const { isDragging } = useFileDrop();

  useWorkspaceSync();

  useEffect(() => {
    logger.info("PinLocal Frontend Initialized");
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(theme);
  }, [theme]);

  const hasActiveWorkspace = !!config?.active;

  return (
    <TooltipProvider delayDuration={400}>
      {isLoading ? (
        <div className="flex h-screen w-screen items-center justify-center bg-background text-foreground">
          <div className="flex flex-col items-center gap-3">
            <div className="w-8 h-px loader-bar" />
            <p className="text-[10px] font-medium uppercase tracking-[0.25em] text-muted-foreground/50">PinLocal</p>
          </div>
        </div>
      ) : !config?.active ? (
        <WelcomePage />
      ) : (
        <MainContent />
      )}

      <FileDropOverlay isVisible={isDragging && hasActiveWorkspace} />
      
      <Toaster
        position="bottom-right"
        expand={true}
        theme={theme as "light" | "dark"}
        visibleToasts={6}
        icons={{
          success: <CheckCircle2 className="h-4 w-4 text-emerald-500" />,
          error: <AlertCircle className="h-4 w-4 text-destructive" />,
          info: <Info className="h-4 w-4 text-muted-foreground" />,
          loading: <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />,
        }}
      />
    </TooltipProvider>
  );
}


export default App;
