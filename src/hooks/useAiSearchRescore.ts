import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { tauriApi } from "@/lib/tauri";
import type { Image as ImageType, ScoredImage } from "@/types";

type UseAiSearchRescoreArgs = {
  boardId?: number | null;
  aiReady: boolean;
};

export function useAiSearchRescore({ boardId, aiReady }: UseAiSearchRescoreArgs) {
  const [searchQuery, setSearchQuery] = useState("");
  const [rescoreQuery, setRescoreQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ImageType[] | null>(null);
  const [lastScoredImages, setLastScoredImages] = useState<ScoredImage[] | null>(null);
  const [isSearching, setIsSearching] = useState(false);

  const searchTimeout = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => {
    let active = true;
    clearTimeout(searchTimeout.current);

    searchTimeout.current = setTimeout(async () => {
      if (!searchQuery.trim()) {
        if (active) {
          setSearchResults(null);
          setLastScoredImages(null);
          setRescoreQuery("");
        }
        return;
      }

      setIsSearching(true);
      try {
        if (aiReady) {
          const results = await tauriApi.aiSearch(searchQuery, boardId ?? null);
          if (!active) return;
          setLastScoredImages(results);
          setSearchResults(results.map((r) => r.image));
        } else {
          if (!active) return;
          toast.error("Please initialize AI System and Load Model first.");
          setSearchResults(null);
          setLastScoredImages(null);
        }
      } catch (err) {
        if (!active) return;
        console.error("Search failed:", err);
        toast.error("Search failed");
      } finally {
        if (active) {
          setIsSearching(false);
        }
      }
    }, 300);

    return () => {
      active = false;
      clearTimeout(searchTimeout.current);
    };
  }, [searchQuery, aiReady, boardId]);

  const handleRescore = async () => {
    if (!rescoreQuery.trim() || !lastScoredImages) return;

    setIsSearching(true);
    try {
      const results = await tauriApi.aiRescore(rescoreQuery, lastScoredImages);
      setLastScoredImages(results);
      setSearchResults(results.map((r) => r.image));
      toast.success("Results rescored");
    } catch {
      toast.error("Rescore failed");
    } finally {
      setIsSearching(false);
    }
  };

  return {
    searchQuery,
    setSearchQuery,
    rescoreQuery,
    setRescoreQuery,
    searchResults,
    lastScoredImages,
    isSearching,
    handleRescore,
  };
}

