import { create } from "zustand";
import type {
  AppSettings,
  ChatMessage,
  LanguagePreference,
  LegalReference,
  ModelStatus,
} from "../lib/types";

interface ChatState {
  sessionId: string | null;
  messages: ChatMessage[];
  streamingContent: string;
  isStreaming: boolean;
  modelStatus: ModelStatus;
  settings: AppSettings;
  error: string | null;
  setSessionId: (id: string | null) => void;
  setMessages: (messages: ChatMessage[]) => void;
  addMessage: (message: ChatMessage) => void;
  beginStream: () => void;
  appendStreamToken: (token: string) => void;
  endStream: (final?: {
    id: string;
    sessionId: string;
    content: string;
    createdAt: string;
    references: LegalReference[];
  }) => void;
  setModelStatus: (status: ModelStatus) => void;
  setSettings: (settings: AppSettings) => void;
  setLanguage: (language: LanguagePreference) => void;
  setError: (error: string | null) => void;
  clearMessages: () => void;
}

const defaultSettings: AppSettings = {
  modelPath: null,
  nCtx: 4096,
  nThreads: null,
  language: "auto",
  maxTokens: 1024,
  hasSeenOnboarding: false,
};

const defaultModelStatus: ModelStatus = {
  loaded: false,
  loading: false,
  waking: false,
  path: null,
  name: null,
  error: null,
  templateSource: null,
};

export const useChatStore = create<ChatState>((set) => ({
  sessionId: null,
  messages: [],
  streamingContent: "",
  isStreaming: false,
  modelStatus: defaultModelStatus,
  settings: defaultSettings,
  error: null,
  setSessionId: (sessionId) => set({ sessionId }),
  setMessages: (messages) => set({ messages }),
  addMessage: (message) =>
    set((state) => ({ messages: [...state.messages, message] })),
  beginStream: () => set({ isStreaming: true, streamingContent: "", error: null }),
  appendStreamToken: (token) =>
    set((state) => ({ streamingContent: state.streamingContent + token })),
  endStream: (final) =>
    set((state) => {
      if (!final) {
        return { isStreaming: false, streamingContent: "" };
      }
      return {
        isStreaming: false,
        streamingContent: "",
        sessionId: final.sessionId,
        messages: [
          ...state.messages,
          {
            id: final.id,
            sessionId: final.sessionId,
            role: "assistant",
            content: final.content,
            createdAt: final.createdAt,
            references: final.references,
          },
        ],
      };
    }),
  setModelStatus: (modelStatus) => set({ modelStatus }),
  setSettings: (settings) => set({ settings }),
  setLanguage: (language) =>
    set((state) => ({ settings: { ...state.settings, language } })),
  setError: (error) => set({ error }),
  clearMessages: () =>
    set({ messages: [], sessionId: null, streamingContent: "", error: null }),
}));
