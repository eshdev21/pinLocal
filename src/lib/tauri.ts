// src/lib/tauri.ts
// ALL invoke() calls live here. Nothing else calls invoke() directly.

import { invoke } from '@tauri-apps/api/core'
import type { Config, Board, Image, PaginatedImages, AiConfig, ModelId, AiMode, AiRuntimeStatus, ScoredImage, WorkspaceStatus } from '../types'

export const tauriApi = {
  // workspace
  getConfig: () => invoke<Config>('get_config'),
  getWorkspaceStatus: () => invoke<WorkspaceStatus>('get_workspace_status'),
  setActiveWorkspace: (id: string) => invoke<void>('set_active_workspace', { id }),
  addWorkspace: (name: string) => invoke<void>('add_workspace', { name }),
  renameWorkspace: (id: string, name: string) => invoke<void>('rename_workspace', { id, name }),
  removeWorkspace: (id: string) => invoke<void>('remove_workspace', { id }),
  addFoldersToWorkspace: (paths: string[]) => invoke<void>('add_folders_to_workspace', { paths }),
  removeBoardFromWorkspace: (boardId: number) => invoke<void>('remove_board_from_workspace', { id: null, boardId }),
  setLoggingEnabled: (enabled: boolean) => invoke<void>('set_logging_enabled', { enabled }),
  openLogsFolder: () => invoke<void>('open_logs_folder'),
  clearLogs: () => invoke<void>('clear_logs'),
  cleanupOrphanedBoards: () => invoke<number>('cleanup_orphaned_boards'),

  // scan
  scanWorkspace: () => invoke<void>('scan_workspace'),

  // boards
  getBoards: () => invoke<Board[]>('get_boards'),
  createBoard: (name: string) => invoke<void>('create_board', { name }),
  deleteBoard: (boardId: number) => invoke<void>('delete_board', { boardId }),

  // images
  getImages: (boardId: number | null, page: number, pageSize: number, sortBy?: string, sortOrder?: string) =>
    invoke<PaginatedImages>('get_images', { boardId, page, pageSize, sortBy, sortOrder }),
  getImage: (id: number) => invoke<Image>('get_image', { id }),
  deleteImage: (imageId: number) => invoke<void>('delete_image', { imageId }),
  openInExplorer: (path: string) => invoke<void>('open_in_explorer', { path }),
  
  importImages: (boardId: number, filePaths: string[]) => 
    invoke<{ imported: number }>('import_images', { boardId, filePaths }),

  // ai search
  getAiConfig: () => invoke<AiConfig>('get_ai_config'),
  setAiEnabled: (enabled: boolean) => invoke<void>('set_ai_enabled', { enabled }),
  setAiMode: (mode: AiMode) => invoke<void>('set_ai_mode', { value: mode }),
  setVenvPath: (path: string | null) => invoke<void>('set_venv_path', { value: path }),
  setAiModel: (model: ModelId) => invoke<void>('set_ai_model', { value: model }),
  setAiHardware: (hardware: string) => invoke<void>('set_ai_hardware', { value: hardware }),
  setCudaVersion: (version: string) => invoke<void>('set_cuda_version', { value: version }),
  setPythonVersion: (version: string) => invoke<void>('set_python_version', { value: version }),
  setLinkMode: (mode: string) => invoke<void>('set_link_mode', { value: mode }),
  setUseAppdataModels: (useAppdata: boolean) => invoke<void>('set_use_appdata_models', { value: useAppdata }),
  setupSiglip: () => invoke<void>('setup_siglip'),
  killSidecar: () => invoke<void>('kill_sidecar'),
  loadModel: () => invoke<void>('load_model'),
  getAiRuntimeStatus: () => invoke<AiRuntimeStatus>('get_ai_runtime_status'),
  generateEmbeddings: () => invoke<number>('generate_embeddings'),
  cancelIndexing: () => invoke<void>('cancel_indexing'),
  resetEmbeddings: () => invoke<void>('reset_embeddings'),
  selectVenvPath: () => invoke<string | null>('select_venv_path'),
  aiSearch: (query: string, boardId?: number | null) =>
    invoke<ScoredImage[]>('ai_search', { query, boardId: boardId ?? null }),
  aiRescore: (query: string, previousResults: ScoredImage[]) =>
    invoke<ScoredImage[]>('ai_rescore', { query, previousResults }),
  getAppStatus: () => invoke<any>('get_app_status'),
}
