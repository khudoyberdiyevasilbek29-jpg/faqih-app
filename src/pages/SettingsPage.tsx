import { useEffect, useState } from "react";
import {
  Activity,
  ChevronDown,
  ChevronUp,
  Download,
  FolderOpen,
  Save,
} from "lucide-react";
import {
  downloadDefaultModel,
  getModelStatus,
  getSystemStats,
  pickGgufFile,
  saveSettings,
  setModelPath,
} from "../lib/api";
import {
  LANGUAGE_LABELS,
  type AppSettings,
  type LanguagePreference,
  type SystemStats,
} from "../lib/types";
import { useChatStore } from "../store/chatStore";
import { Button } from "../components/shared/Button";
import { StatusBadge, modelStatusTone } from "../components/shared/StatusBadge";

const LANGUAGE_OPTIONS: LanguagePreference[] = ["auto", "uz", "ru", "en"];

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function MeterBar({
  label,
  percent,
  detail,
}: {
  label: string;
  percent: number;
  detail: string;
}) {
  const clamped = Math.max(0, Math.min(100, percent));
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-xs">
        <span className="text-surface-soft">{label}</span>
        <span className="text-surface-faint">{detail}</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-surface-elevated">
        <div
          className="h-full rounded-full bg-accent transition-[width] duration-soft"
          style={{ width: `${clamped}%` }}
        />
      </div>
    </div>
  );
}

export function SettingsPage() {
  const settings = useChatStore((s) => s.settings);
  const modelStatus = useChatStore((s) => s.modelStatus);
  const setSettings = useChatStore((s) => s.setSettings);
  const setModelStatus = useChatStore((s) => s.setModelStatus);

  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [statsError, setStatsError] = useState<string | null>(null);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [redownloading, setRedownloading] = useState(false);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    let cancelled = false;

    async function refreshStats() {
      try {
        const next = await getSystemStats();
        if (!cancelled) {
          setStats(next);
          setStatsError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setStatsError(err instanceof Error ? err.message : String(err));
        }
      }
    }

    void refreshStats();
    const timer = window.setInterval(() => {
      void refreshStats();
    }, 3000);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  async function chooseModel() {
    setError(null);
    const path = await pickGgufFile();
    if (!path) return;
    try {
      const status = await setModelPath(path);
      setModelStatus(status);
      const next = { ...draft, modelPath: path };
      setDraft(next);
      setSettings(next);
      setMessage("Maxsus model yo‘li saqlandi.");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function redownload() {
    setError(null);
    setMessage(null);
    setRedownloading(true);
    try {
      const status = await downloadDefaultModel(true);
      setModelStatus(status);
      const next = { ...draft, modelPath: status.path };
      setDraft(next);
      setSettings(next);
      setMessage("AI modeli qayta yuklab olindi.");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRedownloading(false);
    }
  }

  async function persist() {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      await saveSettings(draft);
      setSettings(draft);
      const status = await getModelStatus();
      setModelStatus(status);
      setMessage("Sozlamalar saqlandi. Til keyingi xabardan boshlab qo‘llanadi.");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  const threadValue = draft.nThreads ?? navigator.hardwareConcurrency ?? 4;
  const tone = modelStatusTone(modelStatus);
  const ramPercent = stats
    ? (stats.usedMemoryBytes / Math.max(1, stats.totalMemoryBytes)) * 100
    : 0;

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-6 p-8">
      <section className="panel-card space-y-3">
        <h2 className="text-sm font-semibold text-surface-ink">AI modeli</h2>
        <p className="text-sm text-surface-soft">
          Asosiy model birinchi ochilishda avtomatik yuklab olinadi va shu
          kompyuterda saqlanadi. Oddiy foydalanish uchun qo‘shimcha sozlash
          shart emas.
        </p>
        <div className="glass relative px-3.5 py-2.5 text-xs text-surface-soft break-all">
          <span className="relative z-[1]">
            {draft.modelPath ?? "Model hali o‘rnatilmagan"}
          </span>
        </div>
        <Button
          type="button"
          disabled={redownloading}
          onClick={() => void redownload()}
        >
          <Download className="h-4 w-4" />
          {redownloading ? "Yuklab olinmoqda…" : "Modelni qayta yuklab olish"}
        </Button>

        <div className="border-t border-surface-border pt-3">
          <button
            type="button"
            className="flex w-full items-center justify-between text-xs font-medium text-surface-faint hover:text-surface-soft"
            onClick={() => setAdvancedOpen((v) => !v)}
          >
            <span>Kengaytirilgan (mutaxassislar uchun)</span>
            {advancedOpen ? (
              <ChevronUp className="h-3.5 w-3.5" />
            ) : (
              <ChevronDown className="h-3.5 w-3.5" />
            )}
          </button>
          {advancedOpen ? (
            <div className="mt-3 space-y-2">
              <p className="text-xs leading-relaxed text-surface-faint">
                Allaqachon kompyuteringizda boshqa AI model fayli bo‘lsa, uni
                qo‘lda tanlashingiz mumkin. Bu oddiy foydalanuvchilar uchun
                tavsiya etilmaydi.
              </p>
              <Button
                type="button"
                variant="ghost"
                onClick={() => void chooseModel()}
              >
                <FolderOpen className="h-4 w-4" />
                Maxsus model faylini tanlash…
              </Button>
            </div>
          ) : null}
        </div>
      </section>

      <section className="panel-card space-y-4">
        <h2 className="text-sm font-semibold text-surface-ink">Runtime</h2>

        <label className="block space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-surface-soft">Context window</span>
            <span className="text-surface-faint">{draft.nCtx}</span>
          </div>
          <input
            type="range"
            min={1024}
            max={8192}
            step={512}
            value={draft.nCtx}
            onChange={(e) =>
              setDraft((prev) => ({ ...prev, nCtx: Number(e.target.value) }))
            }
            className="w-full accent-[#c96442]"
          />
        </label>

        <label className="block space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-surface-soft">Thread count</span>
            <span className="text-surface-faint">{threadValue}</span>
          </div>
          <input
            type="range"
            min={1}
            max={Math.max(8, navigator.hardwareConcurrency || 8)}
            step={1}
            value={threadValue}
            onChange={(e) =>
              setDraft((prev) => ({
                ...prev,
                nThreads: Number(e.target.value),
              }))
            }
            className="w-full accent-[#c96442]"
          />
        </label>

        <label className="block space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-surface-soft">Max tokens</span>
            <span className="text-surface-faint">{draft.maxTokens}</span>
          </div>
          <input
            type="range"
            min={128}
            max={2048}
            step={64}
            value={draft.maxTokens}
            onChange={(e) =>
              setDraft((prev) => ({
                ...prev,
                maxTokens: Number(e.target.value),
              }))
            }
            className="w-full accent-[#c96442]"
          />
        </label>
      </section>

      <section className="panel-card space-y-3">
        <h2 className="text-sm font-semibold text-surface-ink">Response language</h2>
        <p className="text-sm text-surface-soft">
          Keyingi xabardan boshlab qo‘llanadi — qayta ishga tushirish shart emas.
        </p>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {LANGUAGE_OPTIONS.map((lang) => {
            const active = draft.language === lang;
            return (
              <button
                key={lang}
                type="button"
                onClick={() => setDraft((prev) => ({ ...prev, language: lang }))}
                className={`relative px-3 py-2.5 text-sm transition-[filter,transform] duration-soft ease-glass ${
                  active
                    ? "glass-primary glass-interactive"
                    : "glass glass-interactive text-surface-soft"
                }`}
              >
                <span className="relative z-[1]">{LANGUAGE_LABELS[lang]}</span>
              </button>
            );
          })}
        </div>
      </section>

      <section className="panel-card space-y-3">
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4 text-accent" />
          <h2 className="text-sm font-semibold text-surface-ink">System resources</h2>
        </div>
        <p className="text-sm text-surface-soft">
          Local RAM/CPU snapshot via sysinfo. Idle LLM unload after 10 minutes
          frees process memory until the next request (Uyg&apos;onmoqda…).
        </p>
        {stats ? (
          <div className="space-y-3">
            <MeterBar
              label="System RAM"
              percent={ramPercent}
              detail={`${formatBytes(stats.usedMemoryBytes)} / ${formatBytes(stats.totalMemoryBytes)}`}
            />
            <MeterBar
              label="CPU"
              percent={stats.cpuUsagePercent}
              detail={`${stats.cpuUsagePercent.toFixed(0)}% · ${stats.cpuCount} cores`}
            />
            <p className="text-xs text-surface-faint">
              Faqih AI process: {formatBytes(stats.processMemoryBytes)} · free{" "}
              {formatBytes(stats.availableMemoryBytes)}
            </p>
          </div>
        ) : (
          <p className="text-xs text-surface-faint">
            {statsError ?? "Reading system stats…"}
          </p>
        )}
      </section>

      <section className="panel-card space-y-3">
        <h2 className="text-sm font-semibold text-surface-ink">Model status</h2>
        <StatusBadge
          tone={tone}
          label={
            modelStatus.waking
              ? "Uyg'onmoqda..."
              : modelStatus.loaded
                ? "Ready"
                : modelStatus.loading
                  ? "Loading"
                  : modelStatus.error
                    ? "Error"
                    : modelStatus.path
                      ? "Idle (unloaded)"
                      : "Idle"
          }
          detail={modelStatus.name ?? modelStatus.path ?? undefined}
        />
        <dl className="grid gap-2 text-xs text-surface-soft">
          <div className="flex justify-between gap-4">
            <dt>Template</dt>
            <dd className="text-surface-soft">
              {modelStatus.templateSource ?? "—"}
            </dd>
          </div>
          <div className="flex justify-between gap-4">
            <dt>Error</dt>
            <dd className="text-right text-surface-soft">
              {modelStatus.error ?? "—"}
            </dd>
          </div>
        </dl>
      </section>

      <div className="flex flex-wrap items-center gap-3">
        <Button type="button" onClick={() => void persist()} disabled={saving}>
          <Save className="h-4 w-4" />
          {saving ? "Saving…" : "Save settings"}
        </Button>
        {message ? <p className="text-xs text-emerald-700">{message}</p> : null}
        {error ? <p className="text-xs text-rose-700">{error}</p> : null}
      </div>
    </div>
  );
}
