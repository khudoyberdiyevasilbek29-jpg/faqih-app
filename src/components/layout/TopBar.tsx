import { ModelStatusIndicator } from "../shared/ModelStatusIndicator";
import { useChatStore } from "../../store/chatStore";
import { useUiStore } from "../../store/uiStore";

const titles: Record<string, { title: string; subtitle: string }> = {
  chat: {
    title: "Suhbat",
    subtitle: "Qonun bo‘yicha tinch savol-javob",
  },
  "document-analysis": {
    title: "Hujjat tahlili",
    subtitle: "Ichki ziddiyatlarni topish",
  },
  settings: {
    title: "Sozlamalar",
    subtitle: "Model va til sozlamalari",
  },
};

export function TopBar() {
  const page = useUiStore((s) => s.page);
  const modelStatus = useChatStore((s) => s.modelStatus);
  const meta = titles[page] ?? titles.chat!;

  return (
    <header className="flex h-16 shrink-0 items-center justify-between border-b border-surface-border bg-surface-base px-8">
      <div>
        <h1 className="text-[15px] font-semibold text-surface-ink">{meta.title}</h1>
        <p className="text-xs text-surface-faint">{meta.subtitle}</p>
      </div>
      <ModelStatusIndicator status={modelStatus} />
    </header>
  );
}
