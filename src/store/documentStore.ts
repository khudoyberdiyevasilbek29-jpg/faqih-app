import { create } from "zustand";
import type { ContradictionReport } from "../lib/types";

interface DocumentState {
  filePath: string | null;
  fileName: string | null;
  analyzing: boolean;
  report: ContradictionReport | null;
  error: string | null;
  setFile: (path: string, name: string) => void;
  setAnalyzing: (analyzing: boolean) => void;
  setReport: (report: ContradictionReport | null) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

export const useDocumentStore = create<DocumentState>((set) => ({
  filePath: null,
  fileName: null,
  analyzing: false,
  report: null,
  error: null,
  setFile: (filePath, fileName) =>
    set({ filePath, fileName, report: null, error: null }),
  setAnalyzing: (analyzing) => set({ analyzing }),
  setReport: (report) => set({ report, analyzing: false, error: null }),
  setError: (error) => set({ error, analyzing: false }),
  reset: () =>
    set({
      filePath: null,
      fileName: null,
      analyzing: false,
      report: null,
      error: null,
    }),
}));
