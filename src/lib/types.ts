/** Shared domain types for Faqih AI — mirrors Rust serde camelCase DTOs. */

export type AppPage = "chat" | "document-analysis" | "settings";

export type LanguagePreference = "auto" | "uz" | "ru" | "en";

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system" | string;
  content: string;
  createdAt: string;
  references: LegalReference[];
}

export interface LegalReference {
  id: string;
  code: string;
  article: string;
  title: string;
  excerpt: string;
  score?: number | null;
}

export interface ChatSession {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface SendChatResponse {
  sessionId: string;
  userMessage: ChatMessage;
  assistantMessage: ChatMessage;
  stopReason: string;
}

export interface ChatTokenEvent {
  token: string;
}

export interface ChatStreamEndEvent {
  stopReason: string;
}

export interface ModelStatus {
  loaded: boolean;
  loading: boolean;
  waking: boolean;
  path: string | null;
  name: string | null;
  error: string | null;
  templateSource: string | null;
}

/** Cold-start / wake stages emitted by Rust (`engine-progress`). */
export type EngineStageId =
  | "checking_files"
  | "loading_llm"
  | "loading_embeddings"
  | "connecting_db"
  | "ready"
  | "error";

export interface EngineProgress {
  stage: string;
  stageId: EngineStageId | string;
  percent: number | null;
  ready: boolean;
  error: string | null;
}

export const ENGINE_STAGE_LABELS: Record<string, string> = {
  checking_files: "Model fayllari tekshirilmoqda…",
  loading_llm: "Til modeli yuklanmoqda…",
  loading_embeddings: "Embedding modeli yuklanmoqda…",
  connecting_db: "Huquqiy bazaga ulanilmoqda…",
  ready: "Tayyor",
  error: "Yuklash xatosi",
};

export function engineStageLabel(progress: EngineProgress | null): string {
  if (!progress) return "Yuklanmoqda…";
  if (progress.error) return progress.error;
  return ENGINE_STAGE_LABELS[progress.stageId] ?? ENGINE_STAGE_LABELS[progress.stage] ?? "Yuklanmoqda…";
}

export interface SystemStats {
  totalMemoryBytes: number;
  usedMemoryBytes: number;
  availableMemoryBytes: number;
  processMemoryBytes: number;
  cpuUsagePercent: number;
  cpuCount: number;
}

export interface ModelSetupStatus {
  ready: boolean;
  downloading: boolean;
  expectedBytes: number;
  localPath: string | null;
  partialBytes: number;
}

export interface ModelDownloadProgress {
  downloadedBytes: number;
  totalBytes: number;
  bytesPerSecond: number;
  etaSeconds: number | null;
  phase: string;
  message: string | null;
}

export interface AppSettings {
  modelPath: string | null;
  nCtx: number;
  nThreads: number | null;
  language: LanguagePreference;
  maxTokens: number;
  hasSeenOnboarding: boolean;
}

export interface ContradictionItem {
  statementA: string;
  statementB: string;
  explanation: string;
}

export interface ContradictionReport {
  summary: string;
  contradictions: ContradictionItem[];
  documentLengthChars: number;
  wasChunked: boolean;
  analysisFailed?: boolean;
}

export interface HelloWorldResponse {
  message: string;
  appName: string;
  version: string;
  offline: true;
}

export const LANGUAGE_LABELS: Record<LanguagePreference, string> = {
  auto: "Avto",
  uz: "Oʻzbek",
  ru: "Русский",
  en: "English",
};

export const CHAT_SUGGESTIONS = [
  "Mehnat shartnomasini bekor qilish tartibi",
  "Mulk huquqi nima",
  "Ish vaqti va dam olish qoidalari",
] as const;

/** Max upload size for document analysis (20 MB). */
export const MAX_DOCUMENT_BYTES = 20 * 1024 * 1024;
export const ACCEPTED_DOCUMENT_EXT = [".pdf", ".docx", ".txt", ".md"] as const;
