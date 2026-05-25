import { useImages } from "@/hooks/useImages";
import { useBoards } from "@/hooks/useBoards";
import { useUIStore } from "@/stores/uiStore";
import { PageHeader } from "@/components/layout/PageHeader";
import { ImageGridContainer } from "@/components/images/ImageGridContainer";
import { useAiEngine } from "@/hooks/useAiEngine";
import { useAiSearchRescore } from "@/hooks/useAiSearchRescore";
import { Sparkles } from "lucide-react";
import { useEffect } from "react";
import { logger } from "@/lib/logger";
import { SortControls } from "@/components/images/SortControls";

export default function BoardPage() {
  const { activeBoardId, setActiveBoardId } = useUIStore();
  const { data: boards } = useBoards();
  const { data, isLoading, fetchNextPage, hasNextPage, isFetchingNextPage } = useImages(activeBoardId);
  
  const { aiReady } = useAiEngine();
  
  const { 
    searchQuery, setSearchQuery, 
    rescoreQuery, setRescoreQuery, 
    searchResults, isSearching, 
    handleRescore 
  } = useAiSearchRescore({ boardId: activeBoardId, aiReady });

  useEffect(() => {
    if (activeBoardId !== null && boards && !boards.some((b) => b.id === activeBoardId)) {
      setActiveBoardId(null);
    }
  }, [activeBoardId, boards, setActiveBoardId]);

  useEffect(() => {
    logger.info(`Viewing Board: ${activeBoardId || 'root'}`);
  }, [activeBoardId]);

  const board = boards?.find((b) => b.id === activeBoardId);
  if (!board) return null;

  const images = searchResults ?? data?.pages.flatMap((p) => p.images) ?? [];

  return (
    <div className="flex flex-col h-full">
      <PageHeader
        title={board.name === "." ? "Home" : board.name}
        imageCount={board.image_count}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        rescoreQuery={rescoreQuery}
        setRescoreQuery={setRescoreQuery}
        handleRescore={handleRescore}
        hasSearchResults={!!searchResults}
      >
        {/* Sort Controls */}
        {!searchQuery ? (
          <SortControls />
        ) : (
          <div className="relevance-badge animate-fade-in">
            <Sparkles size={11} className="text-blue-500/50" />
            <span>Relevance</span>
          </div>
        )}
      </PageHeader>

      <div className="flex-1 overflow-y-auto px-5 pb-10">
        <ImageGridContainer
          images={images}
          isLoading={isLoading}
          isSearching={isSearching}
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          fetchNextPage={fetchNextPage}
          boardName={board.name}
        />
      </div>
    </div>
  );
}

