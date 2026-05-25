import { useAiEngine } from "@/hooks/useAiEngine";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

interface AiStatusDotsProps {
  showLabel?: boolean;
  className?: string;
}

export function AiStatusDots({ showLabel = true, className }: AiStatusDotsProps) {
  const { aiReady, aiEnabled } = useAiEngine();

  if (!aiEnabled) return null;

  const statusLabel = aiReady ? "AI Online" : "AI Offline";
  const dotColor = aiReady 
    ? "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]" 
    : "bg-muted-foreground/30";

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={cn("w-1.5 h-1.5 rounded-full transition-colors duration-500", dotColor)} />
        </TooltipTrigger>
        <TooltipContent side="top">
          {aiReady ? "AI Model Loaded" : "AI Engine Offline"}
        </TooltipContent>
      </Tooltip>
      
      {showLabel && (
        <span className="text-[10px] font-medium text-muted-foreground/60 uppercase tracking-tight">
          {statusLabel}
        </span>
      )}
    </div>
  );
}

