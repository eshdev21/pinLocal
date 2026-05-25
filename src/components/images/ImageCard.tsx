import { Image as ImageType } from "@/types";
import { getThumbUrl } from "@/lib/utils";
import { useUIStore } from "@/stores/uiStore";
import { Loader2 } from "lucide-react";

interface ImageCardProps {
  image: ImageType;
  contextImages?: any[];
  isSearch?: boolean;
}

export default function ImageCard({ image, contextImages, isSearch }: ImageCardProps) {
  const openLightbox = useUIStore((s) => s.openLightbox);
  const hasThumb = !!image.thumb_path;

  return (
    <div
      className="group relative w-full h-full overflow-hidden rounded-xl bg-secondary/20 cursor-zoom-in ring-1 ring-border/60 hover:ring-border transition-all duration-200"
      onClick={() => openLightbox(image.id, contextImages, isSearch)}
    >
      {hasThumb ? (
        <img
          src={getThumbUrl(image.thumb_path!)}
          alt={image.filename}
          className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-[1.02] select-none animate-in fade-in duration-500"
          loading="lazy"
        />
      ) : (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-secondary/40">
          <Loader2 className="w-4 h-4 text-muted-foreground/20 animate-spin" />
          {image.thumbnail_status === 'generating' && (
            <span className="text-[10px] text-muted-foreground/40 font-medium uppercase tracking-tighter">Generating</span>
          )}
          {image.thumbnail_status === 'failed' && (
            <span className="text-[10px] text-destructive/40 font-medium uppercase tracking-tighter">Failed</span>
          )}
        </div>
      )}

      {/* Hover overlay */}
      <div className="absolute inset-x-0 bottom-0 px-3 py-2.5 bg-gradient-to-t from-black/70 via-black/30 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none">
        <p className="text-white text-[11px] font-medium truncate leading-tight">{image.filename}</p>
        {(image as any).caption && (
          <p className="text-white/50 text-[10px] mt-0.5 line-clamp-1 leading-tight">{(image as any).caption}</p>
        )}
      </div>
    </div>
  );
}
