import type { ReactNode } from "react";

type StatusTone = "ready" | "loading" | "error" | "idle";

interface StatusBadgeProps {
  tone: StatusTone;
  label: string;
  detail?: ReactNode;
}

const dotClass: Record<StatusTone, string> = {
  ready: "bg-emerald-600",
  loading: "bg-amber-600 animate-pulse",
  error: "bg-rose-600",
  idle: "bg-surface-faint",
};

export function StatusBadge({ tone, label, detail }: StatusBadgeProps) {
  return (
    <div className="glass relative inline-flex items-center gap-2 rounded-glass px-3.5 py-1.5 text-xs text-surface-soft">
      <span
        className={`relative z-[1] h-1.5 w-1.5 shrink-0 rounded-full ${dotClass[tone]}`}
      />
      <span className="relative z-[1] font-medium text-surface-ink">{label}</span>
      {detail ? (
        <span className="relative z-[1] text-surface-faint">{detail}</span>
      ) : null}
    </div>
  );
}

export function modelStatusTone(status: {
  loaded: boolean;
  loading: boolean;
  waking?: boolean;
  error: string | null;
}): StatusTone {
  if (status.waking) return "loading";
  if (status.loaded) return "ready";
  if (status.loading) return "loading";
  if (status.error) return "error";
  return "idle";
}
