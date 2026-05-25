// src/types/index.ts

export interface Workspace {
  id: string
  name: string
  board_ids: number[]
  folder_paths: string[]
}

export interface BackgroundTask {
  id: string
  task_type: string
  status: string
  message: string | null
  progress: number
  total: number
  updated_at: number
}

export interface Config {
  active: string | null
  workspaces: Workspace[]
  logging_enabled: boolean
  ai: AiConfig
  active_tasks: BackgroundTask[]
}

export interface Board {
  id: number
  name: string
  path: string
  cover_image: string | null
  image_count: number
  created_at: number
  updated_at: number
}

export interface Image {
  id: number
  filename: string
  path: string
  board_id: number
  board_name: string
  thumb_path: string | null
  thumbnail_status: 'pending' | 'generating' | 'ready' | 'failed'
  width: number
  height: number
  size_bytes: number
  mtime: number
  created_at: number
  caption?: string
  is_missing: boolean
}

export interface ScanResult {
  boards_found: number
  images_found: number
  thumbnails_generated: number
  duration_ms: number
}

export type ModelId = 'siglip2_so400m' | 'siglip2_base'
export type AiMode = 'Auto' | 'Manual'
export type HardwareType = 'Auto' | 'Nvidia' | 'Amd' | 'Cpu'
export type CudaVersion = 'V11_8' | 'V12_1' | 'V12_4' | 'V12_6' | 'V13_0'
export type PythonVersion = 'Auto' | 'V3_10' | 'V3_11' | 'V3_12'
export type UvLinkMode = 'Copy' | 'Hardlink' | 'Symlink'

export interface AiConfig {
  enabled: boolean
  mode: AiMode
  venv_path: string | null
  model: ModelId
  hardware: HardwareType
  cuda_version: CudaVersion
  python_version: PythonVersion
  link_mode: UvLinkMode
  use_appdata_models: boolean
}

export interface ModelStatus {
  downloaded: boolean
  vision_path: string | null
  text_path: string | null
  tokenizer_path: string | null
}

export interface AiRuntimeStatus {
  python_ready: boolean
  model_ready: boolean
  is_running: boolean
  is_loaded: boolean
}

export interface DownloadStatus {
  model_id: ModelId
  model: string
  file: string
  percent: number
  active: boolean
  done: boolean
  error: string | null
}

export interface ScoredImage {
  image: Image
  score: number
}


export interface PaginatedImages {
  images: Image[]
  total: number
}

export interface WorkspaceStatus {
  active_workspace_id: string | null;
  ai_config: AiConfig;
  active_tasks: BackgroundTask[];
  is_scanning: boolean;
  ai_engine_status: 'stopped' | 'starting' | 'running' | 'error' | 'disabled';
  ai_model_status: 'unloaded' | 'loading' | 'ready' | 'error';
  pid: number;
}

export interface ThumbnailUpdate {
  id: number
  thumb_path: string
  width: number
  height: number
}
