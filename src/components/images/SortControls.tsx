import { Calendar, ArrowDownAZ, HardDrive, ArrowUp, ArrowDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { useUIStore } from "@/stores/uiStore";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

export function SortControls() {
  const { sortBy, sortOrder, setSortBy, setSortOrder } = useUIStore();

  const sortOptions = [
    { key: "date", icon: Calendar, label: "Date" },
    { key: "name", icon: ArrowDownAZ, label: "Name" },
    { key: "size", icon: HardDrive, label: "Size" },
  ] as const;

  return (
    <div className="sort-group shrink-0 animate-fade-in">
      {sortOptions.map(({ key, icon: Icon, label }) => (
        <Tooltip key={key}>
          <TooltipTrigger asChild>
            <button
              onClick={() => setSortBy(key)}
              className={cn("sort-btn", sortBy === key && "sort-btn-active")}
            >
              <Icon size={13} />
            </button>
          </TooltipTrigger>
          <TooltipContent>Sort by {label}</TooltipContent>
        </Tooltip>
      ))}
      <div className="w-px h-2 bg-border/40 mx-0.5" />
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            onClick={() => setSortOrder(sortOrder === "asc" ? "desc" : "asc")}
            className="sort-btn"
          >
            {sortOrder === "asc" ? <ArrowUp size={13} /> : <ArrowDown size={13} />}
          </button>
        </TooltipTrigger>
        <TooltipContent>{sortOrder === "asc" ? "Ascending" : "Descending"}</TooltipContent>
      </Tooltip>
    </div>
  );
}
