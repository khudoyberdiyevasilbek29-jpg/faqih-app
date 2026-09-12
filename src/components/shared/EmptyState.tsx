import type { ReactNode } from "react";

interface EmptyStateProps {
  title: string;
  description?: string;
  children?: ReactNode;
}

export function EmptyState({ title, description, children }: EmptyStateProps) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-8 py-24 text-center">
      <h2 className="wordmark max-w-xl text-[1.75rem] leading-snug text-surface-ink md:text-[2.1rem]">
        {title}
      </h2>
      {description ? (
        <p className="mt-4 max-w-md text-[15px] leading-relaxed text-surface-soft">
          {description}
        </p>
      ) : null}
      {children ? <div className="mt-10 w-full max-w-xl">{children}</div> : null}
    </div>
  );
}
