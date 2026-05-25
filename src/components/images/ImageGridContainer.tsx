import MasonryGrid from "./MasonryGrid";
import ImageCard from "./ImageCard";
import { Loader2, RefreshCw, Search } from "lucide-react";
import { useInfiniteScroll } from "@/hooks/useInfiniteScroll";
import type { Image } from "@/types";

interface ImageGridContainerProps {
  images: Image[];
  isLoading: boolean;
  isSearching?: boolean;
  isScanning?: boolean;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
  onScan?: () => void;
  emptyTitle?: string;
  emptyDescription?: string;
  boardName?: string;
}

export function ImageGridContainer({
  images,
  isLoading,
  isSearching,
  isScanning,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
  onScan,
  emptyTitle = "No images yet",
  emptyDescription = "Add images to your workspace or run a scan.",
  boardName,
}: ImageGridContainerProps) {
  const { observerRef } = useInfiniteScroll({
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
  });

  if (isLoading || isSearching) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-3 pt-4">
        {[...Array(18)].map((_, i) => (
          <div key={i} className="skeleton aspect-[3/4]" style={{ animationDelay: `${i * 40}ms` }} />
        ))}
      </div>
    );
  }

  if (isScanning) {
    return (
      <div className="h-64 flex flex-col items-center justify-center gap-3 text-center">
        <Loader2 className="animate-spin text-muted-foreground/30 w-5 h-5" />
        <p className="text-[12px] text-muted-foreground/50">Scanning workspace…</p>
      </div>
    );
  }

  if (images.length === 0) {
    return (
      <div className="h-64 flex flex-col items-center justify-center gap-4 text-center">
        <div className="w-10 h-10 bg-secondary rounded-xl flex items-center justify-center">
          {onScan ? <RefreshCw size={16} className="text-muted-foreground/30" /> : <Search size={16} className="text-muted-foreground/30" />}
        </div>
        <div>
          <p className="text-[13px] font-medium mb-1">{emptyTitle}</p>
          <p className="text-[12px] text-muted-foreground/50 max-w-52 leading-relaxed">
            {boardName ? (
              <>Add images to <span className="font-medium">"{boardName}"</span> in your workspace.</>
            ) : (
              emptyDescription
            )}
          </p>
        </div>
        {onScan && (
          <button
            onClick={onScan}
            className="px-4 py-1.5 text-[12px] font-medium bg-foreground text-background rounded-md hover:opacity-90 active:scale-95 transition-all"
          >
            Scan now
          </button>
        )}
      </div>
    );
  }

  return (
    <>
      <div className="pt-4">
        <MasonryGrid
          items={images}
          renderItem={(image: Image) => <ImageCard image={image} contextImages={images} isSearch={isSearching} />}
          getItemSize={(image: Image) => ({ width: image.width || 300, height: image.height || 400 })}
        />
      </div>
      <div ref={observerRef} className="py-16 flex items-center justify-center">
        {isFetchingNextPage ? (
          <Loader2 className="animate-spin text-muted-foreground/30 w-4 h-4" />
        ) : !hasNextPage ? (
          <span className="text-[10px] font-medium text-muted-foreground/20 uppercase tracking-widest">End</span>
        ) : null}
      </div>
    </>
  );
}
