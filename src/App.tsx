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
import { getModelSetupStatus, getModelStatus, getSettings } from "./lib/api";
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

  const [booting, setBooting] = useState(true);
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  // Default true: fail closed until Rust confirms a valid GGUF on disk.
  const [needsSetup, setNeedsSetup] = useState(true);

  const refreshSetup = useCallback(async () => {
    try {
      const [nextSettings, status, setup] = await Promise.all([
        getSettings(),
        getModelStatus(),
        getModelSetupStatus(),
      ]);
      setSettings(nextSettings);
      setModelStatus(status);
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
  }, [setModelStatus, setSettings]);

  useEffect(() => {
    void refreshSetup();
  }, [refreshSetup]);

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
      void getModelStatus()
        .then((status) => {
          if (!cancelled) setModelStatus(status);
        })
        .catch(() => undefined);
    }, 2500);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [booting, needsOnboarding, needsSetup, setModelStatus]);

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
