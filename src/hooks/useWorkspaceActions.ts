import { useState } from "react";
import { tauriApi } from "@/lib/tauri";
import { toast } from "sonner";

export function useWorkspaceActions() {
  const [isScanning, setIsScanning] = useState(false);

  const handleScan = async () => {
    setIsScanning(true);
    try {
      await tauriApi.scanWorkspace();
      toast.success("Scan triggered");
    } catch (error) {
      toast.error("Scan failed");
      console.error("Scan error:", error);
    } finally {
      setIsScanning(false);
    }
  };

  return {
    handleScan,
    isScanning,
  };
}
