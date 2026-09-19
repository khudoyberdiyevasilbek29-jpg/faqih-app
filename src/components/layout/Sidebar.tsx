import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import type { AppPage } from "../../lib/types";
import { useUiStore } from "../../store/uiStore";

const navItems: { id: AppPage; label: string; requiresModel?: boolean }[] = [
  { id: "chat", label: "Suhbat", requiresModel: true },
  { id: "document-analysis", label: "Hujjat tahlili", requiresModel: true },
  { id: "settings", label: "Sozlamalar" },
];

interface SidebarProps {
  /** When false, Chat / Document Analysis are disabled. */
  modelReady?: boolean;
}

export function Sidebar({ modelReady = true }: SidebarProps) {
  const { page, setPage, sidebarCollapsed, toggleSidebar } = useUiStore();

  return (
    <aside
      className={`faqih-sidebar flex h-full shrink-0 flex-col overflow-hidden border-r border-surface-border bg-surface-panel ${
        sidebarCollapsed ? "is-collapsed" : ""
      }`}
    >
      <div className="flex items-center justify-between gap-2 px-5 py-7">
        {!sidebarCollapsed ? (
          <div className="min-w-0">
            <p className="wordmark truncate text-xl text-surface-ink">Faqih AI</p>
            <p className="mt-0.5 truncate text-xs text-surface-faint">by MOND</p>
          </div>
        ) : (
          <p className="wordmark mx-auto text-lg text-surface-ink">F</p>
        )}
        <button
          type="button"
          onClick={toggleSidebar}
          className="rounded-glass p-2 text-surface-faint transition-[color,background-color] duration-150 hover:bg-surface-muted/70 hover:text-surface-ink active:scale-[0.98]"
          aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          {sidebarCollapsed ? (
            <PanelLeftOpen className="h-4 w-4" strokeWidth={1.5} />
          ) : (
            <PanelLeftClose className="h-4 w-4" strokeWidth={1.5} />
          )}
        </button>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3">
        {navItems.map(({ id, label, requiresModel }) => {
          const active = page === id;
          const disabled = Boolean(requiresModel && !modelReady);
          return (
            <button
              key={id}
              type="button"
              disabled={disabled}
              onClick={() => {
                if (disabled) return;
                setPage(id);
              }}
              className={`rounded-card px-4 py-2.5 text-left text-[15px] transition-colors duration-150 ${
                disabled
                  ? "cursor-not-allowed text-surface-faint opacity-45"
                  : active
                    ? "bg-surface-muted font-medium text-surface-ink"
                    : "text-surface-soft hover:bg-surface-muted/70 hover:text-surface-ink"
              }`}
              title={
                disabled
                  ? "Avval AI modelini yuklab oling (Sozlamalar yoki sozlash ekrani)"
                  : label
              }
            >
              {!sidebarCollapsed ? (
                <span className="truncate">{label}</span>
              ) : (
                <span className="block text-center text-sm">{label.charAt(0)}</span>
              )}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
