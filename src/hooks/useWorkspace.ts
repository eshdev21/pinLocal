import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriApi } from "@/lib/tauri";

export const useConfig = () =>
  useQuery({
    queryKey: ["config"],
    queryFn: tauriApi.getConfig,
  });

export const useSetActiveWorkspace = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tauriApi.setActiveWorkspace(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["images"] });
      queryClient.invalidateQueries({ queryKey: ["boards"] });
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });
};

export const useAddWorkspace = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ name }: { name: string }) => tauriApi.addWorkspace(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });
};

export const useRenameWorkspace = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => tauriApi.renameWorkspace(id, name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });
};

export const useRemoveWorkspace = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tauriApi.removeWorkspace(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });
};

export const useAddFoldersToWorkspace = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (paths: string[]) => tauriApi.addFoldersToWorkspace(paths),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["boards"] });
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });
};

export const useRemoveBoardFromWorkspace = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (boardId: number) => tauriApi.removeBoardFromWorkspace(boardId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["boards"] });
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });
};
