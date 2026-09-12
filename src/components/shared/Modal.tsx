import type { ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";

interface ModalProps {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: () => void;
}

export function Modal({ open, title, children, onClose }: ModalProps) {
  return (
    <AnimatePresence>
      {open ? (
        <motion.div
          className="glass-dim fixed inset-0 z-50 flex items-center justify-center p-8"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
          onClick={onClose}
        >
          <motion.div
            className="glass-strong relative w-full max-w-md p-7"
            initial={{ opacity: 0, scale: 0.98 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.98 }}
            transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
            onClick={(event) => event.stopPropagation()}
          >
            <h2 className="relative z-[1] mb-4 text-lg font-semibold text-surface-ink">
              {title}
            </h2>
            <div className="relative z-[1]">{children}</div>
          </motion.div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
