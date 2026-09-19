import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { ChatMessage } from "../../lib/types";
import { SourcesSection } from "./LegalReferenceCard";

interface MessageBubbleProps {
  message: ChatMessage;
  streaming?: boolean;
}

export function MessageBubble({ message, streaming = false }: MessageBubbleProps) {
  const isUser = message.role === "user";

  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[min(100%,36rem)] px-1 py-1 text-[15px] leading-[1.7] ${
          isUser ? "glass relative text-surface-ink" : "text-surface-ink"
        }`}
      >
        {isUser ? (
          <p className="relative z-[1] whitespace-pre-wrap px-3 py-2">{message.content}</p>
        ) : (
          <div className="prose-faqih">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {message.content || (streaming ? "…" : "")}
            </ReactMarkdown>
            {streaming ? (
              <span className="ml-0.5 inline-block h-4 w-1 animate-pulse rounded-sm bg-accent align-middle" />
            ) : null}
          </div>
        )}
        {!isUser && !streaming ? (
          <SourcesSection references={message.references ?? []} />
        ) : null}
      </div>
    </div>
  );
}
