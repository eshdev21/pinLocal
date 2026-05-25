import { useImages } from "@/hooks/useImages";
import { PageHeader } from "@/components/layout/PageHeader";
import { ImageGridContainer } from "@/components/images/ImageGridContainer";
import { useAiEngine } from "@/hooks/useAiEngine";
import { useAiSearchRescore } from "@/hooks/useAiSearchRescore";
import { useWorkspaceActions } from "@/hooks/useWorkspaceActions";
import { Sparkles } from "lucide-react";
import { SortControls } from "@/components/images/SortControls";

export default function HomePage() {
  const { data, isLoading, fetchNextPage, hasNextPage, isFetchingNextPage } = useImages(null);
  
  const { aiReady } = useAiEngine();
  const { handleScan, isScanning } = useWorkspaceActions();
  
  const { 
    searchQuery, setSearchQuery, 
    rescoreQuery, setRescoreQuery, 
    searchResults, isSearching, 
    handleRescore 
  } = useAiSearchRescore({ boardId: null, aiReady });

  const displayImages = searchResults ?? data?.pages.flatMap((p) => p.images) ?? [];
  const totalCount = data?.pages[0]?.total;

  return (
    <div className="flex flex-col h-full">
      <PageHeader
        title="Home"
        imageCount={totalCount}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        rescoreQuery={rescoreQuery}
        setRescoreQuery={setRescoreQuery}
        handleRescore={handleRescore}
        hasSearchResults={!!searchResults}
        onScan={handleScan}
        isScanning={isScanning}
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
          images={displayImages}
          isLoading={isLoading}
          isSearching={isSearching}
          isScanning={isScanning}
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          fetchNextPage={fetchNextPage}
          onScan={handleScan}
        />
      </div>
    </div>
  );
}
