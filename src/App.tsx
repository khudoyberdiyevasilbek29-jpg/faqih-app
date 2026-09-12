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
import { useChatStore } from "./store/chatStore";
import { useUiStore } from "./store/uiStore";
import { LoadingStates } from "./components/shared/LoadingStates";

function App() {
  const page = useUiStore((s) => s.page);
  const settings = useChatStore((s) => s.settings);
  const setSettings = useChatStore((s) => s.setSettings);
  const setModelStatus = useChatStore((s) => s.setModelStatus);

  const [booting, setBooting] = useState(true);
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  const [needsSetup, setNeedsSetup] = useState(false);

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
      setNeedsSetup(!setup.ready);
    } catch {
      setNeedsOnboarding(false);
      setNeedsSetup(false);
    } finally {
      setBooting(false);
    }
  }, [setModelStatus, setSettings]);

  useEffect(() => {
    void refreshSetup();
  }, [refreshSetup]);

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

  if (needsOnboarding || !settings.hasSeenOnboarding) {
    return (
      <OnboardingPage
        onComplete={() => {
          setNeedsOnboarding(false);
        }}
      />
    );
  }

  if (needsSetup) {
    return (
      <ModelSetupPage
        onReady={() => {
          setNeedsSetup(false);
          void refreshSetup();
        }}
      />
    );
  }

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
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar />
        <MainPanel context={context}>{content}</MainPanel>
      </div>
    </div>
  );
}

export default App;
