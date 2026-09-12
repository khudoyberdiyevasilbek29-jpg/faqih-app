import { Loader2 } from "lucide-react";

interface LoadingStatesProps {
  label?: string;
  className?: string;
}

export function LoadingStates({
  label = "Yuklanmoqda…",
  className = "",
}: LoadingStatesProps) {
  return (
    <div className={`flex items-center gap-2 text-sm text-surface-soft ${className}`}>
      <Loader2 className="h-4 w-4 animate-spin text-accent" strokeWidth={1.5} />
      <span>{label}</span>
    </div>
  );
}
