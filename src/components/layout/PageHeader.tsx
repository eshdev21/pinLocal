import { Search, X, Sparkles, RefreshCw } from "lucide-react";
import { cn } from "@/lib/utils";
import { useAiEngine } from "@/hooks/useAiEngine";
import { toast } from "sonner";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

interface PageHeaderProps {
  title: string;
  imageCount?: number;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  rescoreQuery?: string;
  setRescoreQuery?: (q: string) => void;
  handleRescore?: () => void;
  hasSearchResults?: boolean;
  onScan?: () => void;
  isScanning?: boolean;
  children?: React.ReactNode;
  showSort?: boolean;
}

export function PageHeader({
  title,
  imageCount,
  searchQuery,
  setSearchQuery,
  rescoreQuery,
  setRescoreQuery,
  handleRescore,
  hasSearchResults,
  onScan,
  isScanning,
  children,
}: PageHeaderProps) {
  const { aiReady } = useAiEngine();

  return (
    <header className="page-header shrink-0">
      {/* Left: Title & Count */}
      <div className="flex items-center gap-2 shrink-0">
        <h1 className="text-[13px] font-semibold text-foreground truncate max-w-[150px]">{title}</h1>
        {imageCount !== undefined && (
          <span className="text-[11px] text-muted-foreground/40 shrink-0 tabular-nums ml-1">
            {imageCount}
          </span>
        )}
      </div>

      <div className="flex-1" />

      {/* Right: Search, Sort/Children, Sync */}
      <div className="flex items-center gap-3 shrink-0">
        {/* Rescore Input - Only shows when search results are active */}
        {hasSearchResults && setRescoreQuery && (
          <div className="flex items-center gap-1.5 animate-in fade-in slide-in-from-left-2 duration-300">
            <div className="relative flex items-center">
              <input
                type="text"
                value={rescoreQuery}
                onChange={(e) => setRescoreQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleRescore?.()}
                placeholder="Refine…"
                className="search-input pr-7 w-32"
              />
              <Tooltip>
                <TooltipTrigger asChild>
                  <button onClick={handleRescore} className="absolute right-2 text-blue-500/50 hover:text-blue-500">
                    <Sparkles size={10} />
                  </button>
                </TooltipTrigger>
                <TooltipContent>AI Refine</TooltipContent>
              </Tooltip>
            </div>
            <div className="w-px h-3 bg-border/40 mx-0.5" />
          </div>
        )}

        {/* Main Search */}
        <div className="relative flex items-center">
          <Search size={11} className="absolute left-2.5 text-muted-foreground/40 pointer-events-none" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => {
              if (!aiReady) {
                toast.error("Load AI model in Settings first");
                return;
              }
              setSearchQuery(e.target.value);
            }}
            placeholder={aiReady ? "Search images…" : "Search (AI required)"}
            className="search-input pl-7 w-48"
          />
          {aiReady && !searchQuery && (
            <Sparkles size={10} className="absolute right-2.5 text-blue-500/40 pointer-events-none" />
          )}
          {searchQuery && (
            <button onClick={() => setSearchQuery("")} className="absolute right-2.5 text-muted-foreground/40 hover:text-foreground">
              <X size={11} />
            </button>
          )}
        </div>

        {/* Custom Controls (e.g. Sort) */}
        {children}

        {/* Sync Button */}
        {onScan && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onScan}
                disabled={isScanning}
                className="p-1.5 rounded text-muted-foreground hover:text-foreground hover:bg-secondary/60 transition-colors disabled:opacity-40 ml-1"
              >
                <RefreshCw size={13} className={cn(isScanning && "animate-spin")} />
              </button>
            </TooltipTrigger>
            <TooltipContent>Scan Workspace</TooltipContent>
          </Tooltip>
        )}
      </div>
    </header>
  );
}
