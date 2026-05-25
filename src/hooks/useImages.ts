import { useMutation, useQueryClient, useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useUIStore } from "@/stores/uiStore";
import { tauriApi } from "@/lib/tauri";
import { useConfig } from "./useWorkspace";

export const useImages = (boardId: number | null, pageSize = 40) => {
  const sortBy = useUIStore((s) => s.sortBy);
  const sortOrder = useUIStore((s) => s.sortOrder);
  const { data: config } = useConfig();
  const activePath = config?.active || "none";
  
  return useInfiniteQuery({
    queryKey: ["images", activePath, boardId, sortBy, sortOrder],
    queryFn: ({ pageParam = 1 }) => tauriApi.getImages(boardId, pageParam, pageSize, sortBy, sortOrder),
    initialPageParam: 1,
    getNextPageParam: (lastPage, allPages) => {
      if (lastPage.images.length === 0) return undefined;
      const fetchedCount = allPages.reduce((acc, page) => acc + page.images.length, 0);
      return fetchedCount < lastPage.total ? allPages.length + 1 : undefined;
    },
  });
};

export const useDeleteImage = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (imageId: number) => tauriApi.deleteImage(imageId),
    onSuccess: () => {
      // Invalidate is just a fallback; the backend event 'fs:changed' will also trigger it
      queryClient.invalidateQueries({ queryKey: ["images"] });
      queryClient.invalidateQueries({ queryKey: ["boards"] });
    },
  });
};

export const useImage = (imageId: number | null) =>
  useQuery({
    queryKey: ["image", imageId],
    queryFn: () => (imageId ? tauriApi.getImage(imageId) : Promise.reject("No ID")),
    enabled: !!imageId,
  });

