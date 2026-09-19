import type { ReactNode } from "react";

interface ModalProps {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: () => void;
}

/** Lightweight modal: opacity/transform only — no Framer, no layout animation. */
export function Modal({ open, title, children, onClose }: ModalProps) {
  if (!open) return null;

  return (
    <div
      className="glass-dim fixed inset-0 z-50 flex items-center justify-center p-8"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="glass-strong relative w-full max-w-md p-7"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="faqih-modal-title"
      >
        <h2
          id="faqih-modal-title"
          className="relative z-[1] mb-4 text-lg font-semibold text-surface-ink"
        >
          {title}
        </h2>
        <div className="relative z-[1]">{children}</div>
      </div>
    </div>
  );
}
