import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Sidebar } from "./components/layout/Sidebar";
import { TopBar } from "./components/layout/TopBar";
import { MainPanel } from "./components/layout/MainPanel";
import { ChatContextPanel, ChatPage } from "./pages/ChatPage";
import {
  DocumentAnalysisContextPanel,
  DocumentAnalysisPage,
} from "./pages/DocumentAnalysisPage";
import { SettingsPage } from "./pages/SettingsPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { ModelSetupPage } from "./pages/ModelSetupPage";
import {
  getEngineProgress,
  getModelSetupStatus,
  getModelStatus,
  getSettings,
  onEngineProgress,
} from "./lib/api";
import type { AppSettings, ModelStatus } from "./lib/types";
import { useChatStore } from "./store/chatStore";
import { useUiStore } from "./store/uiStore";
import { LoadingStates } from "./components/shared/LoadingStates";

/** Frontend hint that a GGUF path/status exists (disk validation is Rust `setup.ready`). */
function hasConfiguredModel(settings: AppSettings, status: ModelStatus): boolean {
  if (status.loaded || status.loading || status.waking) return true;
  if (status.path) return true;
  if (settings.modelPath) return true;
  return false;
}

function App() {
  const page = useUiStore((s) => s.page);
  const setPage = useUiStore((s) => s.setPage);
  const settings = useChatStore((s) => s.settings);
  const modelStatus = useChatStore((s) => s.modelStatus);
  const setSettings = useChatStore((s) => s.setSettings);
  const setModelStatus = useChatStore((s) => s.setModelStatus);
  const setEngineProgress = useChatStore((s) => s.setEngineProgress);

  const [booting, setBooting] = useState(true);
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  // Default true: fail closed until Rust confirms a valid GGUF on disk.
  const [needsSetup, setNeedsSetup] = useState(true);

  const refreshSetup = useCallback(async () => {
    try {
      const [nextSettings, status, setup, progress] = await Promise.all([
        getSettings(),
        getModelStatus(),
        getModelSetupStatus(),
        getEngineProgress(),
      ]);
      setSettings(nextSettings);
      setModelStatus(status);
      setEngineProgress(progress);
      setNeedsOnboarding(!nextSettings.hasSeenOnboarding);
      // `setup.ready` validates the GGUF file on disk (size + magic).
      setNeedsSetup(!setup.ready);
    } catch {
      // Fail closed — never open Chat with a silent "Model yo'q" state.
      setNeedsOnboarding(false);
      setNeedsSetup(true);
    } finally {
      setBooting(false);
    }
  }, [setEngineProgress, setModelStatus, setSettings]);

  useEffect(() => {
    void refreshSetup();
  }, [refreshSetup]);

  // Live engine warm-up stages (also catch up via getEngineProgress on mount).
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const progress = await getEngineProgress();
        if (!cancelled) setEngineProgress(progress);
      } catch {
        /* ignore until backend is up */
      }
      try {
        unlisten = await onEngineProgress((next) => {
          if (!cancelled) setEngineProgress(next);
        });
      } catch {
        /* web/dev without Tauri */
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setEngineProgress]);

  // If the main shell is up but path/status vanish, return to ModelSetupPage.
  useEffect(() => {
    if (booting || needsOnboarding || needsSetup) return;
    if (!hasConfiguredModel(settings, modelStatus)) {
      setNeedsSetup(true);
    }
  }, [booting, needsOnboarding, needsSetup, settings, modelStatus]);

  useEffect(() => {
    if (needsOnboarding || needsSetup || booting) return;
    let cancelled = false;
    const timer = window.setInterval(() => {
      void Promise.all([getModelStatus(), getEngineProgress()])
        .then(([status, progress]) => {
          if (!cancelled) {
            setModelStatus(status);
            setEngineProgress(progress);
          }
        })
        .catch(() => undefined);
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [booting, needsOnboarding, needsSetup, setEngineProgress, setModelStatus]);

  if (booting) {
    return (
      <div className="flex h-full items-center justify-center bg-surface-base">
        <LoadingStates label="Faqih AI ochilmoqda…" />
      </div>
    );
  }

  // 1) First-run intro
  if (needsOnboarding || !settings.hasSeenOnboarding) {
    return (
      <OnboardingPage
        onComplete={() => {
          setNeedsOnboarding(false);
          void refreshSetup();
        }}
      />
    );
  }

  // 2) Model required before Chat / Document Analysis
  if (needsSetup) {
    return (
      <ModelSetupPage
        onReady={() => {
          setNeedsSetup(false);
          setPage("chat");
          void refreshSetup();
        }}
      />
    );
  }

  const modelReady = hasConfiguredModel(settings, modelStatus);

  let content: ReactNode = <ChatPage />;
  let context: ReactNode | undefined = <ChatContextPanel />;

  if (page === "document-analysis") {
    content = <DocumentAnalysisPage />;
    context = <DocumentAnalysisContextPanel />;
  } else if (page === "settings") {
    content = <SettingsPage />;
    context = undefined;
  }

  return (
    <div className="flex h-full bg-surface-base">
      <Sidebar modelReady={modelReady} />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar />
        <MainPanel context={context}>{content}</MainPanel>
      </div>
    </div>
  );
}

export default App;
