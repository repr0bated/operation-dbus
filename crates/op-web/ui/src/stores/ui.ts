import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface UiState {
  sidebarOpen: boolean;
  theme: 'dark' | 'light' | 'system';
  recentSearches: string[];
  toggleSidebar: () => void;
  setTheme: (theme: UiState['theme']) => void;
  addSearch: (q: string) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarOpen: true,
      theme: 'dark',
      recentSearches: [],
      toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
      setTheme: (theme) => set({ theme }),
      addSearch: (q) => set((s) => ({ 
        recentSearches: [q, ...s.recentSearches.filter(x => x !== q)].slice(0, 10) 
      })),
    }),
    { name: 'ui' }
  )
);
