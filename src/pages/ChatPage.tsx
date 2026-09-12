import { useCallback } from "react";
import {
  onChatToken,
  sendChatMessage,
} from "../lib/api";
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
    error,
    addMessage,
    beginStream,
    appendStreamToken,
    endStream,
    setSessionId,
    setError,
  } = useChatStore();

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
      disabled={!modelStatus.path && !modelStatus.loaded}
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
