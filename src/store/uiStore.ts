import { create } from "zustand";
import type { AppPage } from "../lib/types";

interface UiState {
  page: AppPage;
  sidebarCollapsed: boolean;
  contextPanelOpen: boolean;
  setPage: (page: AppPage) => void;
  toggleSidebar: () => void;
  setContextPanelOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  page: "chat",
  sidebarCollapsed: false,
  contextPanelOpen: true,
  setPage: (page) => set({ page }),
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setContextPanelOpen: (open) => set({ contextPanelOpen: open }),
}));
