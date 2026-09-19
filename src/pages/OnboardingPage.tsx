import { useState } from "react";
import { Scale, MessageCircle, FileSearch, Lock } from "lucide-react";
import { saveSettings } from "../lib/api";
import type { AppSettings } from "../lib/types";
import { useChatStore } from "../store/chatStore";
import { Button } from "../components/shared/Button";

interface OnboardingPageProps {
  onComplete: () => void;
}

const screens = [
  {
    id: "welcome",
    icon: Scale,
    title: "Faqih AI — huquqiy hujjatlaringiz bilan ishlash uchun shaxsiy yordamchi",
    body: "Oddiy tilda yordam beradi. Murakkab sozlamalarsiz, tinch va tushunarli interfeys.",
  },
  {
    id: "features",
    icon: MessageCircle,
    title: "Ikki asosiy imkoniyat",
    features: [
      {
        icon: MessageCircle,
        title: "Savol-javob",
        text: "Qonun bo‘yicha savol bering — tushunarli javob va manbalarni oling.",
      },
      {
        icon: FileSearch,
        title: "Hujjat tahlili",
        text: "Shartnoma yoki boshqa hujjatdagi ichki ziddiyatlarni topishga yordam beradi.",
      },
    ],
  },
  {
    id: "privacy",
    icon: Lock,
    title: "Maxfiylik kafolati",
    body: "Barcha hujjatlaringiz faqat sizning kompyuteringizda qoladi, hech qayerga yuborilmaydi.",
  },
] as const;

export function OnboardingPage({ onComplete }: OnboardingPageProps) {
  const settings = useChatStore((s) => s.settings);
  const setSettings = useChatStore((s) => s.setSettings);
  const [index, setIndex] = useState(0);
  const [saving, setSaving] = useState(false);

  const screen = screens[index]!;
  const isLast = index === screens.length - 1;

  async function finish() {
    setSaving(true);
    const next: AppSettings = { ...settings, hasSeenOnboarding: true };
    try {
      await saveSettings(next);
    } catch {
      // Still continue locally if Tauri unavailable (vite preview).
    }
    setSettings(next);
    setSaving(false);
    onComplete();
  }

  function next() {
    if (isLast) {
      void finish();
      return;
    }
    setIndex((i) => i + 1);
  }

  return (
    <div className="flex h-full items-center justify-center bg-surface-base px-8 py-12">
      <div className="flex w-full max-w-xl flex-col">
        <p className="wordmark text-2xl text-surface-ink">Faqih AI</p>
        <p className="mt-1 text-sm text-surface-faint">by MOND</p>

        <div className="mt-16 min-h-[280px]">
          <div className="glass relative mb-8 flex h-14 w-14 items-center justify-center text-accent">
            <screen.icon className="relative z-[1] h-6 w-6" strokeWidth={1.5} />
          </div>

          <h1 className="wordmark text-[1.75rem] leading-snug text-surface-ink md:text-[2rem]">
            {screen.title}
          </h1>

          {"body" in screen && screen.body ? (
            <p className="mt-5 max-w-md text-[17px] leading-relaxed text-surface-soft">
              {screen.body}
            </p>
          ) : null}

          {"features" in screen && screen.features ? (
            <div className="mt-8 space-y-6">
              {screen.features.map((feature) => (
                <div key={feature.title} className="flex gap-4">
                  <feature.icon
                    className="mt-0.5 h-5 w-5 shrink-0 text-accent"
                    strokeWidth={1.5}
                  />
                  <div>
                    <p className="text-base font-semibold text-surface-ink">
                      {feature.title}
                    </p>
                    <p className="mt-1 text-[15px] leading-relaxed text-surface-soft">
                      {feature.text}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          ) : null}
        </div>

        <div className="mt-12 flex items-center justify-between gap-4">
          <div className="flex items-center gap-2" aria-label="Sahifalar">
            {screens.map((s, i) => (
              <button
                key={s.id}
                type="button"
                aria-label={`Sahifa ${i + 1}`}
                onClick={() => setIndex(i)}
                className={`h-2 rounded-full transition-[width,background-color] duration-150 ${
                  i === index
                    ? "w-6 bg-accent"
                    : "w-2 bg-surface-border hover:bg-surface-faint"
                }`}
              />
            ))}
          </div>

          <div className="flex items-center gap-3">
            {!isLast ? (
              <button
                type="button"
                onClick={() => void finish()}
                disabled={saving}
                className="px-2 py-2 text-sm text-surface-faint transition-colors duration-150 hover:text-surface-soft"
              >
                O&apos;tkazib yuborish
              </button>
            ) : null}
            <Button type="button" onClick={next} disabled={saving}>
              {isLast ? "Boshlash" : "Davom etish"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
