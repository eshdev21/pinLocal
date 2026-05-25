import { useEffect, useRef } from "react";

interface AiTerminalProps {
  logs: string[];
}

export function AiTerminal({ logs }: AiTerminalProps) {
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "instant" });
  }, [logs]);

  return (
    <div className="px-4 py-3 border-b border-border/40 animate-fade-in">
      <div className="bg-background rounded-md border border-border/40 font-mono text-[10.5px] h-48 overflow-y-auto p-3 space-y-0.5">
        {logs.length === 0 ? (
          <div className="text-muted-foreground/30 italic text-center pt-8">No output yet</div>
        ) : (
          logs.map((line, i) => (
            <div 
              key={i} 
              className="text-muted-foreground/70 hover:text-muted-foreground transition-colors leading-relaxed break-all"
            >
              {line}
            </div>
          ))
        )}
        <div ref={logEndRef} />
      </div>
    </div>
  );
}
