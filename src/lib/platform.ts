/** Runtime platform hints for CSS / interaction tuning. */

export type AppPlatform = "windows" | "macos" | "linux" | "unknown";

export function detectPlatform(): AppPlatform {
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac OS X|Macintosh/i.test(ua)) return "macos";
  if (/Linux/i.test(ua)) return "linux";
  return "unknown";
}

/** Adds `platform-windows` / `platform-macos` / `platform-linux` on <html>. */
export function applyPlatformClass(): AppPlatform {
  const platform = detectPlatform();
  const root = document.documentElement;
  root.classList.remove("platform-windows", "platform-macos", "platform-linux");
  if (platform !== "unknown") {
    root.classList.add(`platform-${platform}`);
  }
  root.dataset.platform = platform;
  return platform;
}

export function isWindowsPlatform(): boolean {
  return detectPlatform() === "windows";
}
