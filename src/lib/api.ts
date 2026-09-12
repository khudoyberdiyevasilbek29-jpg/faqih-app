/**
 * Single typed Tauri invoke() + event wrapper.
 * Frontend MUST call Rust exclusively through this module.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AppSettings,
  ChatMessage,
  ChatSession,
  ChatStreamEndEvent,
  ChatTokenEvent,
  ContradictionReport,
  HelloWorldResponse,
  ModelDownloadProgress,
  ModelSetupStatus,
  ModelStatus,
  SendChatResponse,
  SystemStats,
} from "./types";

export const CHAT_TOKEN_EVENT = "chat-token";
export const CHAT_STREAM_END_EVENT = "chat-stream-end";
export const MODEL_DOWNLOAD_PROGRESS_EVENT = "model-download-progress";

export function helloWorld(name: string): Promise<HelloWorldResponse> {
  return invoke<HelloWorldResponse>("hello_world", { name });
}

export function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

export function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export function getModelStatus(): Promise<ModelStatus> {
  return invoke<ModelStatus>("get_model_status");
}

export function setModelPath(path: string): Promise<ModelStatus> {
  return invoke<ModelStatus>("set_model_path", { path });
}

export function getSystemStats(): Promise<SystemStats> {
  return invoke<SystemStats>("get_system_stats");
}

export function getModelSetupStatus(): Promise<ModelSetupStatus> {
  return invoke<ModelSetupStatus>("get_model_setup_status");
}

export function downloadDefaultModel(force = false): Promise<ModelStatus> {
  return invoke<ModelStatus>("download_default_model", { force });
}

export async function onModelDownloadProgress(
  handler: (event: ModelDownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<ModelDownloadProgress>(MODEL_DOWNLOAD_PROGRESS_EVENT, (event) => {
    handler(event.payload);
  });
}

export function sendChatMessage(
  message: string,
  sessionId?: string | null,
): Promise<SendChatResponse> {
  return invoke<SendChatResponse>("send_chat_message", {
    message,
    sessionId: sessionId ?? null,
  });
}

export function getChatHistory(sessionId: string): Promise<ChatMessage[]> {
  return invoke<ChatMessage[]>("get_chat_history", { sessionId });
}

export function listChatSessions(): Promise<ChatSession[]> {
  return invoke<ChatSession[]>("list_chat_sessions");
}

export function analyzeDocument(filePath: string): Promise<ContradictionReport> {
  return invoke<ContradictionReport>("analyze_document", { filePath });
}

/** Subscribe to streamed LLM tokens for the active chat generation. */
export async function onChatToken(
  handler: (event: ChatTokenEvent) => void,
): Promise<UnlistenFn> {
  return listen<ChatTokenEvent>(CHAT_TOKEN_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function onChatStreamEnd(
  handler: (event: ChatStreamEndEvent) => void,
): Promise<UnlistenFn> {
  return listen<ChatStreamEndEvent>(CHAT_STREAM_END_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function pickGgufFile(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "GGUF model", extensions: ["gguf"] }],
  });
  if (typeof selected === "string") return selected;
  return null;
}

export async function pickDocumentFile(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Documents",
        extensions: ["pdf", "docx", "txt", "md"],
      },
    ],
  });
  if (typeof selected === "string") return selected;
  return null;
}
