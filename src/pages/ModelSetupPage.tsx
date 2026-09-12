import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  downloadDefaultModel,
  getModelSetupStatus,
  getSettings,
  onModelDownloadProgress,
} from "../lib/api";
import type { ModelDownloadProgress } from "../lib/types";
import { useChatStore } from "../store/chatStore";
import { Button } from "../components/shared/Button";

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function formatEta(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) {
    return "hisoblanmoqda…";
  }
  if (seconds < 60) return `taxminan ${Math.max(1, Math.round(seconds))} soniya`;
  const mins = Math.ceil(seconds / 60);
  return `taxminan ${mins} daqiqa`;
}

interface ModelSetupPageProps {
  onReady: () => void;
}

export function ModelSetupPage({ onReady }: ModelSetupPageProps) {
  const setSettings = useChatStore((s) => s.setSettings);
  const setModelStatus = useChatStore((s) => s.setModelStatus);

  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ModelDownloadProgress | null>(null);
  const [partialBytes, setPartialBytes] = useState(0);

  useEffect(() => {
    let cancelled = false;
    void getModelSetupStatus()
      .then((status) => {
        if (cancelled) return;
        setPartialBytes(status.partialBytes);
        if (status.ready) onReady();
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [onReady]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onModelDownloadProgress((event) => {
      setProgress(event);
      if (event.phase === "error" && event.message) {
        setError(event.message);
        setDownloading(false);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const percent = useMemo(() => {
    const total = progress?.totalBytes || 986_048_768;
    const done = progress?.downloadedBytes ?? partialBytes;
    if (!total) return 0;
    return Math.min(100, Math.round((done / total) * 100));
  }, [partialBytes, progress]);

  async function startDownload() {
    setError(null);
    setDownloading(true);
    setProgress({
      downloadedBytes: partialBytes,
      totalBytes: 986_048_768,
      bytesPerSecond: 0,
      etaSeconds: null,
      phase: "starting",
      message: "Yuklab olish boshlanmoqda…",
    });
    try {
      const status = await downloadDefaultModel();
      setModelStatus(status);
      const settings = await getSettings();
      setSettings(settings);
      onReady();
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Modelni yuklab olishda xatolik. Qayta urinib ko‘ring.",
      );
      setDownloading(false);
    }
  }

  return (
    <div className="flex h-full items-center justify-center bg-surface-base px-8 py-12">
      <motion.div
        className="w-full max-w-lg"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.22 }}
      >
        <p className="wordmark text-2xl text-surface-ink">Faqih AI</p>
        <h1 className="mt-10 wordmark text-[1.75rem] leading-snug text-surface-ink">
          Bir martalik sozlash
        </h1>
        <p className="mt-4 text-[17px] leading-relaxed text-surface-soft">
          Faqih AI ishlashi uchun sun&apos;iy intellekt modelini yuklab olish
          kerak, bu bir martalik jarayon. Keyingi ochilishlarda qayta
          so‘ralmaydi — hammasi shu kompyuterda saqlanadi.
        </p>

        {!downloading ? (
          <div className="mt-10 space-y-4">
            <Button type="button" onClick={() => void startDownload()}>
              Modelni yuklab olish
            </Button>
            <p className="text-sm leading-relaxed text-surface-faint">
              Hajmi taxminan 1 GB. Internet tezligiga qarab bir necha daqiqa
              ketishi mumkin. Yuklab olish davomida ilovani yopmang.
            </p>
            {partialBytes > 0 ? (
              <p className="text-sm text-amber-800">
                Avvalgi urinishdan {formatBytes(partialBytes)} saqlangan —
                davom ettirish mumkin.
              </p>
            ) : null}
          </div>
        ) : (
          <div className="mt-10 space-y-4">
            <div className="flex items-center justify-between text-sm text-surface-soft">
              <span>
                {progress?.phase === "verifying"
                  ? "Tekshirilmoqda…"
                  : progress?.phase === "done"
                    ? "Tayyor"
                    : "Yuklab olinmoqda…"}
              </span>
              <span className="text-surface-faint">{percent}%</span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-surface-muted">
              <motion.div
                className="h-full rounded-full bg-accent"
                initial={false}
                animate={{ width: `${percent}%` }}
                transition={{ duration: 0.2 }}
              />
            </div>
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-surface-faint">
              <span>
                {formatBytes(progress?.downloadedBytes ?? partialBytes)} /{" "}
                {formatBytes(progress?.totalBytes ?? 986_048_768)}
              </span>
              {progress?.bytesPerSecond ? (
                <span>{formatBytes(progress.bytesPerSecond)}/s</span>
              ) : null}
              <span>Qolgan vaqt: {formatEta(progress?.etaSeconds)}</span>
            </div>
            <p className="text-xs text-surface-faint">
              Yuklab bo‘lgach, fayl avtomatik tekshiriladi
            </p>
          </div>
        )}

        {error ? (
          <div className="glass relative mt-8 space-y-4 p-5">
            <p className="relative z-[1] whitespace-pre-wrap text-sm leading-relaxed text-rose-800 dark:text-rose-200">
              {error}
            </p>
            <div className="relative z-[1]">
              <Button type="button" onClick={() => void startDownload()}>
                Qayta urinish
              </Button>
            </div>
          </div>
        ) : null}
      </motion.div>
    </div>
  );
}
