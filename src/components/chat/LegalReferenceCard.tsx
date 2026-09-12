import { useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { LegalReference } from "../../lib/types";

interface LegalReferenceCardProps {
  reference: LegalReference;
}

export function LegalReferenceCard({ reference }: LegalReferenceCardProps) {
  return (
    <div className="glass relative p-4 text-left">
      <p className="relative z-[1] text-xs font-medium text-accent">
        {reference.code || "Qonun"}
        {reference.article ? ` · ${reference.article}-modda` : ""}
      </p>
      {reference.title ? (
        <p className="relative z-[1] mt-1 text-sm font-medium text-surface-ink">
          {reference.title}
        </p>
      ) : null}
      {reference.excerpt ? (
        <p className="relative z-[1] mt-2 text-xs leading-relaxed text-surface-soft">
          {reference.excerpt}
        </p>
      ) : null}
    </div>
  );
}

interface SourcesSectionProps {
  references: LegalReference[];
}

export function SourcesSection({ references }: SourcesSectionProps) {
  const [open, setOpen] = useState(false);
  if (!references.length) return null;

  return (
    <div className="mt-4 border-t border-surface-border pt-3">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between text-xs font-medium text-surface-faint hover:text-surface-ink"
      >
        <span>Manbalar ({references.length})</span>
        {open ? (
          <ChevronUp className="h-3.5 w-3.5" strokeWidth={1.5} />
        ) : (
          <ChevronDown className="h-3.5 w-3.5" strokeWidth={1.5} />
        )}
      </button>
      {open ? (
        <div className="mt-3 space-y-2">
          {references.map((ref) => (
            <LegalReferenceCard key={ref.id} reference={ref} />
          ))}
        </div>
      ) : null}
    </div>
  );
}
