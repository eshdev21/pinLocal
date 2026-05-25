import { create } from 'zustand'
import type { BackgroundTask, WorkspaceStatus } from '../types'

interface StatusStore {
  activeWorkspaceId: string | null
  isScanning: boolean
  engineStatus: 'stopped' | 'starting' | 'running' | 'error' | 'disabled'
  modelStatus: 'unloaded' | 'loading' | 'ready' | 'error'
  activeTasks: BackgroundTask[]
  pid: number
  aiLogs: string[]
  isTerminalOpen: boolean

  // The unified sync method that handles the backend pulse
  sync: (payload: WorkspaceStatus) => void
  addAiLog: (line: string) => void
  clearAiLogs: () => void
  setTerminalOpen: (open: boolean) => void
}

export const useStatusStore = create<StatusStore>((set) => ({
  activeWorkspaceId: null,
  isScanning: false,
  engineStatus: 'stopped',
  modelStatus: 'unloaded',
  activeTasks: [],
  pid: 0,
  aiLogs: [],
  isTerminalOpen: false,

  addAiLog: (line) => {
    set((state) => {
      const trimmed = line.trim();
      if (!trimmed) return state;
      
      const isProgress = /%|\[|MB\/s|GB\/s/.test(trimmed);
      const newLogs = [...state.aiLogs];
      
      // If it's a progress line, replace the last line if it was also a progress line
      if (isProgress && newLogs.length > 0 && /%|\[|MB\/s|GB\/s/.test(newLogs[newLogs.length - 1])) {
        newLogs[newLogs.length - 1] = trimmed;
      } else {
        newLogs.push(trimmed);
      }
      
      // Keep only last 200 lines
      return { aiLogs: newLogs.slice(-200) };
    });
  },

  clearAiLogs: () => set({ aiLogs: [] }),
  setTerminalOpen: (open: boolean) => set({ isTerminalOpen: open }),

  sync: (payload) => {
    // We only update if the data actually changed to avoid React re-renders
    set((state) => {
      const updates: Partial<StatusStore> = {}

      if (payload.active_workspace_id !== state.activeWorkspaceId) {
        updates.activeWorkspaceId = payload.active_workspace_id
      }

      if (payload.is_scanning !== state.isScanning) {
        updates.isScanning = payload.is_scanning
      }

      if (payload.ai_engine_status !== state.engineStatus) {
        updates.engineStatus = payload.ai_engine_status
      }

      if (payload.ai_model_status !== state.modelStatus) {
        updates.modelStatus = payload.ai_model_status
      }

      if (payload.pid !== state.pid) {
        updates.pid = payload.pid
      }

      // Compare tasks efficiently: if length or ANY task's update time changed, we update
      const tasksChanged = !state.activeTasks ||
        payload.active_tasks?.length !== state.activeTasks.length ||
        payload.active_tasks.some((t: any, i: number) => t.updated_at !== state.activeTasks[i]?.updated_at);

      if (tasksChanged) {
        updates.activeTasks = payload.active_tasks || []
      }

      if (Object.keys(updates).length > 0) {
        return updates
      }
      return state
    })
  }
}))
