import type { ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";
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
      <AnimatePresence initial={false}>
        {contextPanelOpen && context ? (
          <motion.aside
            className="hidden w-80 shrink-0 overflow-auto border-l border-surface-border bg-surface-panel p-8 lg:block"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          >
            {context}
          </motion.aside>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
