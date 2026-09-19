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
        fast: "150ms",
        soft: "180ms",
        panel: "200ms",
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
        // Softer / cheaper default shadow (was 32px blur).
        glass: "0 1px 0 rgba(44, 42, 38, 0.05), 0 4px 16px rgba(44, 42, 38, 0.04)",
        "glass-dark":
          "0 1px 0 rgba(255,255,255,0.08), 0 4px 16px rgba(0, 0, 0, 0.22)",
      },
    },
  },
  plugins: [],
};
