import { create } from 'zustand'
import { persist } from 'zustand/middleware'

interface LightboxContext {
  images: any[]
  currentIndex: number
  isSearch?: boolean
}

interface UIStore {
  lightbox: { 
    open: boolean; 
    imageId: number | null;
    context: LightboxContext | null;
  }
  openLightbox: (imageId: number, images?: any[], isSearch?: boolean) => void
  closeLightbox: () => void

  activeBoardId: number | null
  setActiveBoardId: (id: number | null) => void

  sortBy: 'date' | 'name' | 'size'
  sortOrder: 'asc' | 'desc'
  setSortBy: (sortBy: 'date' | 'name' | 'size') => void
  setSortOrder: (sortOrder: 'asc' | 'desc') => void

  theme: 'light' | 'dark'
  toggleTheme: () => void
}

export const useUIStore = create<UIStore>()(
  persist(
    (set, get) => ({
      lightbox: { open: false, imageId: null, context: null },
      openLightbox: (imageId, images, isSearch) => {
        let context: LightboxContext | null = null;
        if (images && images.length > 0) {
          const idx = images.findIndex(img => (img.id || img.image?.id) === imageId);
          if (idx !== -1) {
            context = { images, currentIndex: idx, isSearch };
          }
        }
        set({ lightbox: { open: true, imageId, context } });
      },
      closeLightbox: () => set({ lightbox: { open: false, imageId: null, context: null } }),

      activeBoardId: null,
      setActiveBoardId: (id) => set({ activeBoardId: id }),

      sortBy: 'date',
      sortOrder: 'desc',
      setSortBy: (sortBy) => set({ sortBy }),
      setSortOrder: (sortOrder) => set({ sortOrder }),

      theme: 'dark',
      toggleTheme: () => {
        const next = get().theme === 'dark' ? 'light' : 'dark'
        set({ theme: next })
        const root = document.documentElement
        root.classList.remove('light', 'dark')
        root.classList.add(next)
      },
    }),
    {
      name: 'pinlocal-ui',
      partialize: (s) => ({ theme: s.theme, sortBy: s.sortBy, sortOrder: s.sortOrder }),
    }
  )
)
