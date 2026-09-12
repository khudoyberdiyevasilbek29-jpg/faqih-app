import { Loader2 } from "lucide-react";
import type { ModelStatus } from "../../lib/types";
import { StatusBadge, modelStatusTone } from "./StatusBadge";

interface ModelStatusIndicatorProps {
  status: ModelStatus;
}

export function ModelStatusIndicator({ status }: ModelStatusIndicatorProps) {
  const tone = modelStatusTone(status);

  if (status.waking) {
    return (
      <StatusBadge
        tone="loading"
        label="Uyg'onmoqda..."
        detail={<Loader2 className="h-3 w-3 animate-spin text-surface-faint" />}
      />
    );
  }

  if (status.loaded) {
    return <StatusBadge tone="ready" label="Model tayyor" />;
  }

  if (status.loading) {
    return (
      <StatusBadge
        tone="loading"
        label="Yuklanmoqda…"
        detail={<Loader2 className="h-3 w-3 animate-spin text-surface-faint" />}
      />
    );
  }

  if (status.path && !status.error) {
    return <StatusBadge tone="idle" label="Kutish holati" />;
  }

  if (status.error) {
    return <StatusBadge tone="error" label={status.error} />;
  }

  return <StatusBadge tone={tone} label="Model yo‘q" />;
}
