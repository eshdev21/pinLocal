import { ChevronLeft, ChevronRight, ExternalLink, Trash2, X, Info, Loader2 } from "lucide-react";
import { useUIStore } from "@/stores/uiStore";
import { useImages, useDeleteImage, useImage } from "@/hooks/useImages";
import { useConfig } from "@/hooks/useWorkspace";
import { getImageUrl, cn } from "@/lib/utils";
import { useEffect, useState, useCallback, useRef } from "react";
import { tauriApi } from "@/lib/tauri";
import { toast } from "sonner";
import { Dialog, DialogContent, DialogTitle, ConfirmDialog } from "@/components/ui/dialog";

export default function Lightbox() {
  const { lightbox, closeLightbox, activeBoardId } = useUIStore();
  const { data: config } = useConfig();
  const { data: imagesData, fetchNextPage, hasNextPage, isFetchingNextPage } = useImages(activeBoardId);
  const { data: currentImage, isLoading } = useImage(lightbox.imageId);
  const deleteImage = useDeleteImage();
  const [showInfo, setShowInfo] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isImageLoading, setIsImageLoading] = useState(true);

  // Reset loading state when image changes
  const [lastImageId, setLastImageId] = useState(lightbox.imageId);
  if (lastImageId !== lightbox.imageId) {
    setLastImageId(lightbox.imageId);
    setIsImageLoading(true);
  }

  const images = (lightbox.context?.isSearch ? lightbox.context.images : null) ?? 
                 imagesData?.pages?.flatMap((p) => p.images) ?? 
                 lightbox.context?.images ?? 
                 [];
  const currentIndex = images.findIndex((img) => (img.id || img.image?.id) === lightbox.imageId);
  const hasPrev = currentIndex > 0;
  
  // In search mode, we only have what we have. In board mode, we can have more.
  const hasNext = currentIndex >= 0 && (currentIndex < images.length - 1 || (!lightbox.context?.isSearch && hasNextPage));

  // Proactive pre-fetching: Load next page when approaching the end
  useEffect(() => {
    if (lightbox.open && !lightbox.context?.isSearch && hasNextPage && !isFetchingNextPage) {
      if (currentIndex >= images.length - 5) {
        fetchNextPage();
      }
    }
  }, [currentIndex, images.length, hasNextPage, isFetchingNextPage, fetchNextPage, lightbox.open, lightbox.context?.isSearch]);

  const handlePrev = useCallback(() => {
    if (hasPrev) {
      const prevImg = images[currentIndex - 1];
      useUIStore.getState().openLightbox(prevImg.id || prevImg.image?.id, images, lightbox.context?.isSearch);
    }
  }, [currentIndex, hasPrev, images, lightbox.context?.isSearch]);

  const handleNext = useCallback(() => {
    if (currentIndex < images.length - 1) {
      const nextImg = images[currentIndex + 1];
      useUIStore.getState().openLightbox(nextImg.id || nextImg.image?.id, images, lightbox.context?.isSearch);
    } else if (!lightbox.context?.isSearch && hasNextPage && !isFetchingNextPage) {
      // If we are at the very end but more exist, fetch now
      fetchNextPage();
    }
  }, [currentIndex, images, hasNextPage, isFetchingNextPage, fetchNextPage, lightbox.context?.isSearch]);

  const handlersRef = useRef({ handlePrev, handleNext });
  useEffect(() => {
    handlersRef.current = { handlePrev, handleNext };
  });

  useEffect(() => {
    if (!lightbox.open) return;
    let lastScroll = 0;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowLeft") handlersRef.current.handlePrev();
      if (e.key === "ArrowRight") handlersRef.current.handleNext();
    };
    const onWheel = (e: WheelEvent) => {
      if ((e.target as HTMLElement).closest(".overflow-y-auto")) return;
      const now = Date.now();
      if (now - lastScroll < 350 || Math.abs(e.deltaY) < 30) return;
      lastScroll = now;
      if (e.deltaY > 0) handlersRef.current.handleNext();
      else handlersRef.current.handlePrev();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("wheel", onWheel, { passive: true });
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("wheel", onWheel);
    };
  }, [lightbox.open]);

  if (!lightbox.open || !lightbox.imageId || !config?.active) return null;

  const handleDelete = async () => {
    if (!currentImage) return;
    try {
      await deleteImage.mutateAsync(currentImage.id);
      toast.success("Deleted");
      setShowDeleteConfirm(false);
      closeLightbox();
    } catch { toast.error("Delete failed"); }
  };

  const fmtSize = (b: number) => b > 1_048_576 ? `${(b / 1_048_576).toFixed(1)} MB` : `${(b / 1024).toFixed(0)} KB`;

  const showLoader = isLoading || !currentImage || isImageLoading;

  return (
    <Dialog open={lightbox.open} onOpenChange={(open) => !open && closeLightbox()}>
      <DialogContent 
        hideClose
        onClick={closeLightbox}
        className="max-w-[100vw] max-h-[100vh] w-full h-full p-0 bg-transparent border-none shadow-none flex items-center justify-center animate-none"
      >
        {/* We use a custom DialogTitle for accessibility but hide it visually */}
        <DialogTitle className="sr-only">{currentImage?.filename || "Image View"}</DialogTitle>

        {/* Polaroid Frame */}
        <div 
          className="bg-card text-card-foreground shadow-2xl rounded-sm border border-border/40 flex flex-col p-2 pb-14 relative max-w-[98vw] max-h-[98vh] pointer-events-auto overflow-hidden animate-in zoom-in-95 duration-200"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Image Area */}
          <div className="relative flex items-center justify-center overflow-hidden bg-secondary/10 aspect-square w-[80vh] max-w-[calc(100vw-40px)] max-h-[calc(100vh-160px)]">
            {currentImage && (
              <img
                key={currentImage.id}
                src={getImageUrl(currentImage.path)}
                alt={currentImage.filename}
                onLoad={() => setIsImageLoading(false)}
                className={cn(
                  "w-full h-full object-contain select-none transition-all duration-500 ease-out",
                  showLoader ? "opacity-0 scale-95 blur-sm" : "opacity-100 scale-100 blur-0"
                )}
              />
            )}
          </div>

          {/* Polaroid Bottom */}
          {currentImage && (
            <div className="absolute bottom-0 inset-x-0 h-14 px-4 flex items-center justify-between bg-card border-t border-border/10">
              <div className="min-w-0 mr-4">
                <p className="text-[11px] font-bold truncate leading-none mb-0.5">{currentImage.filename}</p>
                <p className="text-[9px] uppercase tracking-tighter text-muted-foreground font-semibold">
                  {currentImage.width}×{currentImage.height} · {fmtSize(currentImage.size_bytes)}
                  {currentImage.board_name && ` · ${currentImage.board_name}`}
                </p>
              </div>

              <div className="flex items-center gap-1">
                <button
                  onClick={() => setShowInfo(!showInfo)}
                  className={cn("p-1.5 rounded transition-colors", showInfo ? "text-primary bg-primary/10" : "text-muted-foreground hover:text-foreground hover:bg-secondary")}
                >
                  <Info size={14} />
                </button>
                <button onClick={() => tauriApi.openInExplorer(currentImage.path)} className="p-1.5 rounded text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                  <ExternalLink size={14} />
                </button>
                <button onClick={() => setShowDeleteConfirm(true)} className="p-1.5 rounded text-muted-foreground hover:text-destructive hover:bg-destructive/5 transition-colors">
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          )}

          {/* Info Overlay */}
          {showInfo && currentImage && (
            <div className="absolute right-0 top-0 bottom-0 w-64 bg-background border-l border-border/40 shadow-xl z-30 flex flex-col animate-in slide-in-from-right duration-200">
              <div className="p-4 border-b border-border/20 flex items-center justify-between">
                <h3 className="font-bold text-[10px] uppercase tracking-widest text-muted-foreground">Properties</h3>
                <button onClick={() => setShowInfo(false)} className="text-muted-foreground hover:text-foreground">
                  <X size={14} />
                </button>
              </div>
              <div className="p-4 space-y-4 overflow-y-auto">
                {[["Path", currentImage.path], ["Modified", new Date(currentImage.mtime * 1000).toLocaleString()]].map(([label, value]) => (
                  <div key={label} className="space-y-1">
                    <p className="text-[9px] font-bold uppercase tracking-widest text-muted-foreground/50">{label}</p>
                    <p className="text-[10px] break-all leading-tight font-medium opacity-80">{value}</p>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <div 
          onClick={(e) => e.stopPropagation()}
          className="fixed bottom-8 right-8 flex items-center gap-3 z-[60] pointer-events-auto animate-fade-in"
        >
          <button
            onClick={handlePrev}
            disabled={!hasPrev}
            className="p-3 rounded-full bg-white text-black shadow-2xl hover:scale-110 active:scale-95 transition-all disabled:opacity-20 disabled:pointer-events-none"
            title="Previous (Left Arrow)"
          >
            <ChevronLeft size={22} strokeWidth={2.5} />
          </button>
          <button
            onClick={handleNext}
            disabled={!hasNext || (currentIndex === images.length - 1 && isFetchingNextPage)}
            className="p-3 rounded-full bg-white text-black shadow-2xl hover:scale-110 active:scale-95 transition-all disabled:opacity-20 disabled:pointer-events-none"
            title="Next (Right Arrow)"
          >
            {currentIndex === images.length - 1 && isFetchingNextPage ? (
              <Loader2 size={22} className="animate-spin" />
            ) : (
              <ChevronRight size={22} strokeWidth={2.5} />
            )}
          </button>
        </div>

        {/* HUD Close (Top Right) */}
        <button 
          onClick={(e) => { e.stopPropagation(); closeLightbox(); }} 
          className="fixed top-8 right-8 z-[60] p-2.5 rounded-full bg-red-500/10 text-red-500 hover:bg-red-500 hover:text-white transition-all shadow-xl backdrop-blur-md border border-red-500/20 group animate-fade-in"
          title="Close (Esc)"
        >
          <X size={20} strokeWidth={2.5} className="group-hover:rotate-90 transition-transform duration-300" />
        </button>

        {/* Background Prefetch: Only if next image is already in memory */}
        {currentIndex < images.length - 1 && (
          <img
            key={`prefetch-${images[currentIndex + 1].id || images[currentIndex + 1].image?.id}`}
            src={getImageUrl(images[currentIndex + 1].path || images[currentIndex + 1].image?.path)}
            className="hidden"
            aria-hidden="true"
          />
        )}
      </DialogContent>
      
      <ConfirmDialog
        isOpen={showDeleteConfirm}
        onClose={() => setShowDeleteConfirm(false)}
        title="Delete Image"
        description="Are you sure you want to delete this image from your disk? This action cannot be undone."
        confirmText="Delete"
        variant="destructive"
        onConfirm={handleDelete}
      />
    </Dialog>
  );
}

