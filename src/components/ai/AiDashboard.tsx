import { useQueryClient } from "@tanstack/react-query";
import { tauriApi } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { Sparkles, X, Terminal, Cpu, Zap, Activity, HardDrive, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { useAiEngine } from "@/hooks/useAiEngine";
import { useStatusStore } from "@/stores/statusStore";
import { AiTerminal } from "./AiTerminal";
import { ConfirmDialog } from "../ui/dialog";
import { SelectWrapper as Select, type SelectOption } from "../ui/select";
import { Switch } from "../ui/switch";
import { RadioGroup, RadioGroupItem } from "../ui/radio-group";
import { Tooltip, TooltipContent, TooltipTrigger, TooltipProvider } from "../ui/tooltip";


const MODEL_OPTIONS: SelectOption[] = [
  { value: "siglip2_so400m", label: "SigLIP 2 SO400M (Large)" },
  { value: "siglip2_base", label: "SigLIP 2 Base (Small)" },
];

const HARDWARE_OPTIONS: SelectOption[] = [
  { value: "Auto", label: "Auto-detect" },
  { value: "Nvidia", label: "NVIDIA (CUDA)" },
  { value: "Amd", label: "AMD (DirectML)" },
  { value: "Cpu", label: "CPU (Slow)" },
];

const CUDA_OPTIONS: SelectOption[] = [
  { value: "V11_8", label: "CUDA 11.8" },
  { value: "V12_1", label: "CUDA 12.1" },
  { value: "V12_4", label: "CUDA 12.4" },
  { value: "V12_6", label: "CUDA 12.6" },
  { value: "V13_0", label: "CUDA 13.0" },
];

const PYTHON_OPTIONS: SelectOption[] = [
  { value: "Auto", label: "Auto-detect" },
  { value: "V3_10", label: "Python 3.10" },
  { value: "V3_11", label: "Python 3.11" },
  { value: "V3_12", label: "Python 3.12" },
];

const LINK_OPTIONS: SelectOption[] = [
  { value: "Copy", label: "Copy (Stable)" },
  { value: "Symlink", label: "Symlink (Fast)" },
  { value: "Hardlink", label: "Hardlink" },
];

export function AiDashboard() {
  const qc = useQueryClient();
  const { config, aiEnabled, isScanning, isLoaded, isRunning, aiEngineStatus } = useAiEngine();

  const [isSetupLoading, setIsSetupLoading] = useState(false);
  const [isModelLoading, setIsModelLoading] = useState(false);
  const [isActionLoading, setIsActionLoading] = useState(false);
  const logs = useStatusStore(s => s.aiLogs);
  const addAiLog = useStatusStore(s => s.addAiLog);
  const clearAiLogs = useStatusStore(s => s.clearAiLogs);
  const isTerminalOpen = useStatusStore(s => s.isTerminalOpen);
  const setTerminalOpen = useStatusStore(s => s.setTerminalOpen);
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  useEffect(() => {
    const subs = [
      listen<string>("ai:log", (e) => addAiLog(e.payload)),
      listen<any>("python:ready", () => {
        qc.invalidateQueries({ queryKey: ["aiConfig"] });
        toast.success("AI environment ready");
      }),
      listen<number>("ai:index-complete", (e) => {
        toast.success(`Indexed ${e.payload} images`);
        qc.invalidateQueries({ queryKey: ["images"] });
      }),
    ];
    return () => { subs.forEach((p) => p.then((f) => f())); };
  }, [qc, addAiLog]);

  if (!config) return null;

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ["aiConfig"] });
    qc.invalidateQueries({ queryKey: ["workspaceStatus"] });
  };

  const run = async (fn: () => Promise<unknown>, successMsg?: string, type: "setup" | "model" | "action" = "action") => {
    if (type === "setup") setIsSetupLoading(true);
    else if (type === "model") setIsModelLoading(true);
    else setIsActionLoading(true);

    setTerminalOpen(true);
    const tid = successMsg ? toast.loading(`Starting: ${successMsg}...`) : null;
    try {
      await fn();
      if (successMsg) {
        if (tid) toast.success(successMsg, { id: tid });
        else toast.success(successMsg);
      }
    } catch (e: any) {
      const msg = e?.toString() ?? "Error";
      if (tid) toast.error(msg, { id: tid });
      else toast.error(msg);
    } finally {
      setIsSetupLoading(false);
      setIsModelLoading(false);
      setIsActionLoading(false);
      invalidate();
    }
  };

  return (
    <section className="space-y-6">
      <h2 className="settings-section-title">AI Semantic Search</h2>

      {/* --- Zone 1: Power & Status --- */}
      <div className="rounded-lg border border-border/40 bg-card shadow-sm overflow-hidden">
        <div className="setting-row px-4">
          <div>
            <p className="setting-label flex items-center gap-2">
              <Sparkles size={13} className={cn("transition-colors", aiEnabled ? "text-blue-500" : "text-muted-foreground/40")} />
              Enable AI search
            </p>
            <p className="setting-desc">Neural image embeddings via SigLIP 2</p>
          </div>
          <Switch checked={aiEnabled} onCheckedChange={async (checked) => {
            await tauriApi.setAiEnabled(checked);
            invalidate();
          }} />
        </div>
      </div>

      <AnimatePresence>
        {aiEnabled && (
          <motion.div
            initial={{ opacity: 0, y: 5 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 5 }}
            className="space-y-6"
          >
            {/* --- Zone 2: Configuration Grid --- */}
            <div className="rounded-lg border border-border/40 bg-card shadow-sm relative">
              <div className="px-4 py-2 border-b border-border/40 bg-muted/5">
                <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/50">
                  Engine Configuration
                </span>
              </div>

              <div className="p-4 grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-5">
                {/* Model Selection */}
                <div className="space-y-1.5">
                  <label className="text-[11px] font-medium text-foreground/80">Model Architecture</label>
                  <Select
                    value={config.model}
                    options={MODEL_OPTIONS}
                    onChange={(v) => run(() => tauriApi.setAiModel(v as any))}
                  />
                  <p className="text-[10px] text-muted-foreground/60 leading-tight">SO400M is more accurate, Base is faster</p>
                </div>

                {/* Environment Mode */}
                <div className="space-y-1.5">
                  <label className="text-[11px] font-medium text-foreground/80">Environment Mode</label>
                  <RadioGroup
                    value={config.mode}
                    onValueChange={(m: string) => run(() => tauriApi.setAiMode(m as any), `Mode: ${m}`)}
                    className="flex bg-secondary rounded-md p-0.5 border border-border/40 gap-0.5 h-8"
                  >
                    {(["Auto", "Manual"] as const).map((m) => (
                      <div key={m} className="flex-1 relative flex items-center">
                        <RadioGroupItem value={m} id={`mode-${m}`} className="sr-only" />
                        <label
                          htmlFor={`mode-${m}`}
                          className={cn(
                            "flex-1 text-center py-1 text-[11px] font-medium rounded cursor-pointer transition-all capitalize",
                            config.mode === m ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"
                          )}
                        >
                          {m}
                        </label>
                      </div>
                    ))}
                  </RadioGroup>
                  <p className="text-[10px] text-muted-foreground/60 leading-tight">Auto manages all dependencies automatically</p>
                </div>

                {config.mode === "Auto" && (
                  <>
                    <div className="md:col-span-2 h-px bg-border/40 my-0.5" />

                    {/* Hardware Acceleration */}
                    <div className="space-y-1.5">
                      <label className="text-[11px] font-medium text-foreground/80 flex items-center gap-1.5">
                        <Cpu size={11} className="text-muted-foreground/40" /> Hardware acceleration
                      </label>
                      <Select
                        value={config.hardware}
                        options={HARDWARE_OPTIONS}
                        onChange={(v) => run(() => tauriApi.setAiHardware(v as any))}
                      />
                    </div>

                    {/* CUDA/Hardware Version */}
                    <div className="space-y-1.5">
                      <label className="text-[11px] font-medium text-foreground/80 flex items-center gap-1.5">
                        <Activity size={11} className="text-muted-foreground/40" /> CUDA version
                      </label>
                      <Select
                        value={config.cuda_version}
                        options={CUDA_OPTIONS}
                        disabled={config.hardware !== "Auto" && config.hardware !== "Nvidia"}
                        onChange={(v) => run(() => tauriApi.setCudaVersion(v as any))}
                      />
                    </div>

                    {/* Python Runtime */}
                    <div className="space-y-1.5">
                      <label className="text-[11px] font-medium text-foreground/80">Python version</label>
                      <Select
                        value={config.python_version}
                        options={PYTHON_OPTIONS}
                        onChange={(v) => run(() => tauriApi.setPythonVersion(v as any))}
                      />
                    </div>

                    {/* UV Link Mode */}
                    <div className="space-y-1.5">
                      <label className="text-[11px] font-medium text-foreground/80">UV link mode</label>
                      <Select
                        value={config.link_mode}
                        options={LINK_OPTIONS}
                        onChange={(v) => run(() => tauriApi.setLinkMode(v as any))}
                      />
                    </div>

                    {/* Model Storage Toggle */}
                    <div className="md:col-span-2 mt-1 pt-3 border-t border-border/40 flex items-center justify-between">
                      <div>
                        <p className="text-[11px] font-medium text-foreground leading-none mb-1 flex items-center gap-1.5">
                          <HardDrive size={11} className="text-muted-foreground/40" />
                          Store models in AppData
                        </p>
                        <p className="text-[10px] text-muted-foreground/60 leading-tight">Isolate multi-GB models inside PinLocal folder</p>
                      </div>
                      <Switch
                        checked={config.use_appdata_models}
                        onCheckedChange={() => run(() => tauriApi.setUseAppdataModels(!config.use_appdata_models))}
                      />
                    </div>
                  </>
                )}

                {/* Manual Mode Venv Path */}
                {config.mode === "Manual" && (
                  <motion.div
                    initial={{ opacity: 0, y: 5 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="md:col-span-2 space-y-2 bg-secondary/30 p-3 rounded-md border border-border/40"
                  >
                    <label className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/60">Python .venv path</label>
                    <div className="flex gap-2">
                      <div className="flex-1 px-3 py-1.5 bg-secondary/60 border border-border/40 rounded-md text-[11px] font-mono text-muted-foreground/60 truncate">
                        {config.venv_path || "Not selected"}
                      </div>
                      <button
                        onClick={async () => {
                          const p = await tauriApi.selectVenvPath();
                          if (p) { await tauriApi.setVenvPath(p); invalidate(); }
                        }}
                        className="px-3 py-1.5 text-[11px] font-medium bg-secondary hover:bg-border rounded-md transition-colors"
                      >
                        Browse
                      </button>
                    </div>
                  </motion.div>
                )}
              </div>
            </div>

            {/* --- Zone 3: Maintenance & Controls --- */}
            <div className="rounded-lg border border-border/40 bg-card shadow-sm overflow-hidden divide-y divide-border/40">
              
              {/* STAGE 1: Foundation (Python Environment) */}
              {config.mode === "Auto" && (
                <div className="px-4 py-3 flex items-center justify-between bg-muted/5">
                  <div>
                    <p className="text-[11px] font-medium text-foreground">Stage 1: AI Foundation</p>
                    <p className="text-[10px] text-muted-foreground/60 leading-tight">
                      {isRunning || aiEngineStatus === "starting" || aiEngineStatus === "stopped" ? "Python environment is healthy" : "Requires one-time setup (Python/Torch)"}
                    </p>
                  </div>
                  <button
                    onClick={() => run(() => tauriApi.setupSiglip(), "AI System Ready", "setup")}
                    disabled={isSetupLoading || isModelLoading}
                    className={cn(
                      "flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-md transition-colors",
                      aiEngineStatus !== "error"
                        ? "bg-secondary hover:bg-border text-muted-foreground" 
                        : "bg-blue-600 hover:bg-blue-500 text-white shadow-sm"
                    )}
                  >
                    <RefreshCw size={12} className={cn(isSetupLoading && "animate-spin")} />
                    {isSetupLoading ? "Working…" : aiEngineStatus !== "stopped" ? "Repair System" : "Initialize System"}
                  </button>
                </div>
              )}

              {/* STAGE 2: Engine (Process & Weights) */}
              <div className="px-4 py-3 flex items-center justify-between">
                <div>
                  <p className="text-[11px] font-medium text-foreground">Stage 2: AI Engine</p>
                  <p className="text-[10px] text-muted-foreground/60 leading-tight">Load model weights into {config.hardware === 'Auto' ? 'GPU' : config.hardware}</p>
                </div>
                <div className="flex items-center gap-2">
                  {isRunning && (
                    <button
                      onClick={() => run(() => tauriApi.killSidecar(), "Engine Stopped", "action")}
                      disabled={isActionLoading}
                      className="p-1.5 text-red-500 hover:bg-red-500/10 rounded-md transition-colors"
                      title="Stop Engine"
                    >
                      <X size={12} />
                    </button>
                  )}
                  <button
                    onClick={() => run(() => tauriApi.loadModel(), "AI Model Ready", "model")}
                    disabled={isSetupLoading || isModelLoading || isLoaded}
                    className={cn(
                      "flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-md transition-colors",
                      isLoaded 
                        ? "bg-emerald-500/10 text-emerald-500 cursor-default" 
                        : "bg-secondary hover:bg-border"
                    )}
                  >
                    <Zap size={11} className={cn(isModelLoading && "animate-pulse")} />
                    {isModelLoading ? "Loading…" : isLoaded ? "Engine Online" : "Load Model"}
                  </button>
                </div>
              </div>

              {/* STAGE 3: Work (The Task) */}
              <div className="px-4 py-3 flex items-center justify-between bg-muted/5">
                <div>
                  <p className="text-[11px] font-medium text-foreground">Stage 3: Image Indexing</p>
                  <p className="text-[10px] text-muted-foreground/60 leading-tight">Scan and embed your local library</p>
                </div>
                <div className="flex items-center gap-3">
                  <button
                    onClick={() => setShowResetConfirm(true)}
                    disabled={aiEngineStatus === "stopped" || isActionLoading || isModelLoading}
                    className="px-3 py-1.5 text-[11px] text-muted-foreground/50 hover:text-muted-foreground transition-colors disabled:opacity-40"
                  >
                    Reset cache
                  </button>
                  <button
                    onClick={async () => {
                      try {
                        addAiLog("INFO: Starting indexer…");
                        setTerminalOpen(true);
                        const count = await tauriApi.generateEmbeddings();
                        if (count === 0) toast.info("Everything is up to date");
                      } catch (e: any) {
                        toast.error(e?.toString() ?? "Indexing failed");
                      }
                    }}
                    disabled={!isLoaded || isScanning || isActionLoading}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium bg-foreground text-background hover:opacity-90 disabled:opacity-40 rounded-md transition-colors"
                  >
                    <Sparkles size={12} className={cn(isScanning && "animate-pulse")} /> 
                    {isScanning ? "Indexing…" : "Start Indexing"}
                  </button>
                </div>
              </div>

              {/* Footer: Console Toggle */}
              <div className="px-4 py-2 bg-card/50 flex justify-end">
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <button
                        onClick={() => setTerminalOpen(!isTerminalOpen)}
                        className={cn(
                          "p-1.5 rounded-md transition-all",
                          isTerminalOpen ? "bg-secondary text-foreground" : "text-muted-foreground/60 hover:text-foreground hover:bg-secondary"
                        )}
                      >
                        <Terminal size={13} />
                      </button>
                    </TooltipTrigger>
                    <TooltipContent>{isTerminalOpen ? "Hide Logs" : "Show Logs"}</TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>

              {/* Log terminal */}
              <AnimatePresence>
                {isTerminalOpen && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    className="overflow-hidden border-t border-border/40"
                  >
                    <AiTerminal logs={logs} />
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            <ConfirmDialog
              isOpen={showResetConfirm}
              onClose={() => setShowResetConfirm(false)}
              title="Reset AI Cache?"
              description="This will clear all generated image embeddings and restart the indexing process. This may take some time depending on your library size."
              confirmText="Reset Cache"
              variant="destructive"
              onConfirm={async () => {
                await tauriApi.resetEmbeddings();
                toast.success("Cache cleared");
                clearAiLogs();
                addAiLog(">>> Cache reset, restarting…");
                setTerminalOpen(true);
                const count = await tauriApi.generateEmbeddings();
                if (count === 0) toast.info("Up to date");
              }}
            />
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  );
}
