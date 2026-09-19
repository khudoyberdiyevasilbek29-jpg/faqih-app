import { useCallback } from "react";
import { onChatToken, sendChatMessage } from "../lib/api";
import { engineStageLabel } from "../lib/types";
import { useChatStore } from "../store/chatStore";
import { ChatWindow } from "../components/chat/ChatWindow";
import { LegalReferenceCard } from "../components/chat/LegalReferenceCard";

export function ChatPage() {
  const {
    sessionId,
    messages,
    streamingContent,
    isStreaming,
    settings,
    modelStatus,
    engineProgress,
    error,
    addMessage,
    beginStream,
    appendStreamToken,
    endStream,
    setSessionId,
    setError,
  } = useChatStore();

  const enginesReady = Boolean(engineProgress?.ready);
  const warming =
    modelStatus.waking ||
    modelStatus.loading ||
    Boolean(engineProgress && !engineProgress.ready && engineProgress.stageId !== "error");
  const engineFailed = Boolean(engineProgress?.error);
  const noModel = !modelStatus.path && !modelStatus.loaded && !settings.modelPath;
  const sendDisabled = warming || engineFailed || noModel || !enginesReady;

  let disabledReason: string | null = null;
  if (engineFailed) {
    disabledReason =
      engineProgress?.error ??
      "Dvigatel yuklanmadi. Dasturni qayta oching yoki model fayllarini qayta yuklang.";
  } else if (warming || !enginesReady) {
    disabledReason = modelStatus.waking
      ? "Model uyg‘onmoqda — biroz kuting…"
      : engineStageLabel(engineProgress);
  } else if (noModel) {
    disabledReason =
      "AI modeli yo‘q. Avval sozlash ekranidan yoki Sozlamalardan modelni yuklang.";
  }

  const handleSend = useCallback(
    async (content: string) => {
      setError(null);
      addMessage({
        id: crypto.randomUUID(),
        sessionId: sessionId ?? "",
        role: "user",
        content,
        createdAt: new Date().toISOString(),
        references: [],
      });
      beginStream();

      let unlistenToken: (() => void) | undefined;
      try {
        unlistenToken = await onChatToken(({ token }) => {
          appendStreamToken(token);
        });

        const result = await sendChatMessage(content, sessionId);
        setSessionId(result.sessionId);
        endStream({
          id: result.assistantMessage.id,
          sessionId: result.sessionId,
          content: result.assistantMessage.content,
          createdAt: result.assistantMessage.createdAt,
          references: result.assistantMessage.references ?? [],
        });
      } catch (err) {
        endStream();
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        unlistenToken?.();
      }
    },
    [
      addMessage,
      appendStreamToken,
      beginStream,
      endStream,
      sessionId,
      setError,
      setSessionId,
    ],
  );

  return (
    <ChatWindow
      messages={messages}
      streamingContent={streamingContent}
      isStreaming={isStreaming}
      language={settings.language}
      onSend={(value) => void handleSend(value)}
      disabled={sendDisabled}
      disabledReason={disabledReason}
      error={error}
    />
  );
}

export function ChatContextPanel() {
  const messages = useChatStore((s) => s.messages);
  const latestAssistant = [...messages]
    .reverse()
    .find((m) => m.role === "assistant" && m.references?.length);

  return (
    <div className="space-y-4">
      <h2 className="text-sm font-semibold text-surface-ink">Manbalar</h2>
      <p className="text-xs leading-relaxed text-surface-soft">
        Oxirgi javobdagi qonun manbalari shu yerda ko‘rinadi.
      </p>
      {latestAssistant?.references?.length ? (
        latestAssistant.references.map((reference) => (
          <LegalReferenceCard key={reference.id} reference={reference} />
        ))
      ) : (
        <p className="text-xs text-surface-faint">Hali manba yo‘q.</p>
      )}
    </div>
  );
}
