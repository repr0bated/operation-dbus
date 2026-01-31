import { create } from 'zustand';

interface QuotaState {
  used: number;
  limit: number;
  warnings: string[];
  setQuota: (used: number, limit: number) => void;
  addWarning: (msg: string) => void;
}

export const useQuotaStore = create<QuotaState>((set) => ({
  used: 0,
  limit: 1000,
  warnings: [],
  setQuota: (used, limit) => set({ used, limit }),
  addWarning: (msg) => set((s) => ({ warnings: [...s.warnings, msg] })),
}));
