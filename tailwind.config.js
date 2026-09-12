/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "media",
  theme: {
    extend: {
      colors: {
        // Claude-like warm neutrals (light-first; dark via CSS vars / media)
        surface: {
          base: "var(--surface-base)",
          panel: "var(--surface-panel)",
          elevated: "var(--surface-elevated)",
          muted: "var(--surface-muted)",
          border: "var(--surface-border)",
          ink: "var(--surface-ink)",
          soft: "var(--surface-soft)",
          faint: "var(--surface-faint)",
        },
        accent: {
          DEFAULT: "#c96442",
          hover: "#b55535",
          soft: "rgba(201, 100, 66, 0.14)",
          from: "#c96442",
          to: "#c96442",
        },
        // Liquid Glass fills (use with .glass utilities in CSS)
        glass: {
          DEFAULT: "var(--glass-bg)",
          strong: "var(--glass-bg-strong)",
          accent: "var(--glass-accent)",
          border: "var(--glass-border)",
          highlight: "var(--glass-highlight)",
          fallback: "var(--glass-fallback)",
          "fallback-accent": "var(--glass-fallback-accent)",
          dim: "var(--glass-dim)",
        },
      },
      fontFamily: {
        serif: [
          "Source Serif 4",
          "Georgia",
          "Cambria",
          "Times New Roman",
          "serif",
        ],
        sans: ["Source Sans 3", "ui-sans-serif", "system-ui", "sans-serif"],
      },
      borderRadius: {
        card: "14px",
        glass: "18px",
        panel: "22px",
      },
      maxWidth: {
        prose: "42rem",
      },
      transitionDuration: {
        fast: "180ms",
        soft: "240ms",
        panel: "280ms",
      },
      transitionTimingFunction: {
        glass: "cubic-bezier(0.22, 1, 0.36, 1)",
        press: "cubic-bezier(0.34, 1.2, 0.64, 1)",
      },
      backdropBlur: {
        glass: "20px",
        panel: "28px",
      },
      boxShadow: {
        glass: "0 8px 32px rgba(44, 42, 38, 0.06), inset 0 1px 0 rgba(255,255,255,0.35)",
        "glass-dark":
          "0 8px 32px rgba(0, 0, 0, 0.28), inset 0 1px 0 rgba(255,255,255,0.12)",
      },
    },
  },
  plugins: [],
};
