import { useEffect, useRef } from "react";
import { MessageBubble } from "./MessageBubble";
import { InputBar } from "./InputBar";
import { EmptyState } from "../shared/EmptyState";
import { CHAT_SUGGESTIONS, type ChatMessage, type LanguagePreference } from "../../lib/types";

interface ChatWindowProps {
  messages: ChatMessage[];
  streamingContent: string;
  isStreaming: boolean;
  language: LanguagePreference;
  onSend: (value: string) => void;
  disabled?: boolean;
  error?: string | null;
}

export function ChatWindow({
  messages,
  streamingContent,
  isStreaming,
  language,
  onSend,
  disabled,
  error,
}: ChatWindowProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const empty = messages.length === 0 && !isStreaming;

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingContent, isStreaming]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-auto px-8 py-8">
        {empty ? (
          <EmptyState title="Nima bilan yordam bera olaman?">
            <div className="flex flex-wrap justify-center gap-2">
              {CHAT_SUGGESTIONS.map((chip) => (
                <button
                  key={chip}
                  type="button"
                  disabled={disabled}
                  onClick={() => onSend(chip)}
                  className="glass glass-interactive relative px-4 py-2.5 text-sm text-surface-soft disabled:opacity-50"
                >
                  <span className="relative z-[1]">{chip}</span>
                </button>
              ))}
            </div>
          </EmptyState>
        ) : (
          <div className="mx-auto flex max-w-prose flex-col gap-6">
            {messages.map((message) => (
              <MessageBubble key={message.id} message={message} />
            ))}
            {isStreaming ? (
              <MessageBubble
                streaming
                message={{
                  id: "streaming",
                  sessionId: "",
                  role: "assistant",
                  content: streamingContent,
                  createdAt: new Date().toISOString(),
                  references: [],
                }}
              />
            ) : null}
            <div ref={bottomRef} />
          </div>
        )}
      </div>

      <div className="border-t border-surface-border px-8 py-6">
        <div className="mx-auto max-w-prose space-y-3">
          {error ? (
            <div className="glass relative px-4 py-3 text-sm text-rose-800 dark:text-rose-200">
              <p className="relative z-[1]">{error}</p>
            </div>
          ) : null}
          <InputBar
            onSend={onSend}
            disabled={disabled || isStreaming}
            language={language}
          />
        </div>
      </div>
    </div>
  );
}
