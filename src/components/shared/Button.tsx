import type { ButtonHTMLAttributes, ReactNode } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  /** primary/secondary = Liquid Glass; ghost = plain text (no glass) */
  variant?: "primary" | "ghost" | "secondary";
}

export function Button({
  children,
  variant = "primary",
  className = "",
  ...props
}: ButtonProps) {
  const base =
    "inline-flex items-center justify-center gap-2 px-5 py-2.5 text-sm font-medium disabled:pointer-events-none";

  const variants: Record<NonNullable<ButtonProps["variant"]>, string> = {
    primary: "glass-primary glass-interactive",
    secondary: "glass glass-interactive text-surface-ink",
    ghost:
      "rounded-glass px-3 py-2 text-surface-soft transition-[color,background-color] duration-soft ease-glass hover:bg-surface-muted/60 hover:text-surface-ink active:scale-[0.98]",
  };

  return (
    <button className={`${base} ${variants[variant]} ${className}`} {...props}>
      <span className="relative z-[1] inline-flex items-center gap-2">{children}</span>
    </button>
  );
}
