import type { EngineProgress, ModelStatus } from "../../lib/types";
import { engineStageLabel } from "../../lib/types";

interface ModelStatusIndicatorProps {
  status: ModelStatus;
  progress: EngineProgress | null;
}

export function ModelStatusIndicator({
  status,
  progress,
}: ModelStatusIndicatorProps) {
  const waking = status.waking;
  const loading =
    waking ||
    status.loading ||
    Boolean(progress && !progress.ready && progress.stageId !== "error");
  const failed = Boolean(progress?.error) || Boolean(status.error && !loading);
  const ready = Boolean(progress?.ready) && status.loaded && !waking;

  const label = failed
    ? "Yuklash xatosi"
    : ready
      ? "Tayyor"
      : waking
        ? "Uyg‘onmoqda…"
        : progress?.ready && !status.loaded
          ? "Kutish holati"
          : engineStageLabel(progress);

  const percent =
    progress?.percent != null && !failed && !ready
      ? Math.min(100, Math.max(0, progress.percent))
      : ready
        ? 100
        : null;
  const showBar = (loading || ready || waking) && !failed;
  const indeterminate = (loading || waking) && !ready && percent == null;

  const toneClass = failed
    ? "border-rose-500/30"
    : ready
      ? "border-emerald-600/25"
      : "border-surface-border";

  return (
    <div
      className={`glass relative min-w-[11rem] max-w-[18rem] rounded-glass px-3.5 py-2 ${toneClass}`}
      title={failed ? (progress?.error ?? status.error ?? undefined) : label}
    >
      <div className="relative z-[1] flex items-center justify-between gap-2">
        <span className="truncate text-xs font-medium text-surface-ink">{label}</span>
        {loading && !failed ? (
          <span
            className="faqih-spinner h-3 w-3 shrink-0 rounded-full border border-surface-faint border-t-accent"
            aria-hidden
          />
        ) : null}
        {ready ? (
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-600" aria-hidden />
        ) : null}
        {failed ? (
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-rose-600" aria-hidden />
        ) : null}
      </div>
      {showBar ? (
        <div className="relative z-[1] mt-1.5 h-1 overflow-hidden rounded-full bg-surface-muted">
          <div
            className={`h-full rounded-full bg-accent/80 transition-[width] duration-300 ease-out ${
              indeterminate ? "faqih-progress-indeterminate w-1/3" : ""
            }`}
            style={
              indeterminate
                ? undefined
                : { width: `${percent ?? 12}%` }
            }
          />
        </div>
      ) : null}
    </div>
  );
}
