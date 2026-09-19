import type { ReactNode } from "react";
import { useUiStore } from "../../store/uiStore";

interface MainPanelProps {
  children: ReactNode;
  context?: ReactNode;
}

export function MainPanel({ children, context }: MainPanelProps) {
  const contextPanelOpen = useUiStore((s) => s.contextPanelOpen);

  return (
    <div className="flex min-h-0 flex-1">
      <main className="min-w-0 flex-1 overflow-auto">{children}</main>
      {contextPanelOpen && context ? (
        <aside className="hidden w-80 shrink-0 overflow-auto border-l border-surface-border bg-surface-panel p-8 lg:block">
          {context}
        </aside>
      ) : null}
    </div>
  );
}
