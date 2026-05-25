import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriApi } from "@/lib/tauri";

export const useBoards = () =>
  useQuery({
    queryKey: ["boards"],
    queryFn: tauriApi.getBoards,
  });

export const useCreateBoard = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => tauriApi.createBoard(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["boards"] });
    },
  });
};

