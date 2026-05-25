import { useQuery } from "@tanstack/react-query";
import { tauriApi } from "@/lib/tauri";
import { useStatusStore } from "@/stores/statusStore";

export function useAiEngine() {
  const { data: config, isLoading: isConfigLoading } = useQuery({
    queryKey: ["aiConfig"],
    queryFn: tauriApi.getAiConfig,
  });

  const engineStatus = useStatusStore(s => s.engineStatus);
  const modelStatus = useStatusStore(s => s.modelStatus);
  const isScanning = useStatusStore(s => s.isScanning);
  
  const aiEnabled = !!config?.enabled;
  const aiReady = !!(aiEnabled && modelStatus === "ready");
  const isLoaded = modelStatus === "ready";
  const isRunning = engineStatus === "running";

  return {
    config,
    aiEnabled,
    aiReady,
    aiEngineStatus: engineStatus,
    aiModelStatus: modelStatus,
    isScanning,
    isLoaded,
    isRunning,
    isLoading: isConfigLoading,
  };
}
