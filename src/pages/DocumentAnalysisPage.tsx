import { analyzeDocument } from "../lib/api";
import { useChatStore } from "../store/chatStore";
import { useDocumentStore } from "../store/documentStore";
import { Button } from "../components/shared/Button";
import { LoadingStates } from "../components/shared/LoadingStates";
import { DocumentUploader } from "../components/document-analysis/DocumentUploader";
import { ContradictionReportView } from "../components/document-analysis/ContradictionReport";

export function DocumentAnalysisPage() {
  const modelStatus = useChatStore((s) => s.modelStatus);
  const {
    filePath,
    fileName,
    analyzing,
    report,
    error,
    setFile,
    setAnalyzing,
    setReport,
    setError,
    reset,
  } = useDocumentStore();

  const modelReady = Boolean(modelStatus.path || modelStatus.loaded);

  async function runAnalysis() {
    if (!filePath) return;
    setAnalyzing(true);
    setError(null);
    try {
      const result = await analyzeDocument(filePath);
      setReport(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6 p-8">
      <DocumentUploader
        disabled={analyzing || !modelReady}
        onPathSelected={setFile}
        error={error}
      />

      {fileName ? (
        <div className="panel-card flex flex-wrap items-center justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-wide text-surface-faint">
              Tanlangan fayl
            </p>
            <p className="mt-1 text-sm text-surface-ink">{fileName}</p>
          </div>
          <div className="flex gap-2">
            <Button type="button" variant="ghost" onClick={reset} disabled={analyzing}>
              Tozalash
            </Button>
            <Button
              type="button"
              onClick={() => void runAnalysis()}
              disabled={analyzing || !modelReady}
            >
              Tahlil qilish
            </Button>
          </div>
        </div>
      ) : null}

      {analyzing ? (
        <div className="panel-card">
          <LoadingStates label="Hujjat tahlil qilinmoqda…" />
        </div>
      ) : null}

      {report ? <ContradictionReportView report={report} /> : null}
    </div>
  );
}

export function DocumentAnalysisContextPanel() {
  const report = useDocumentStore((s) => s.report);
  return (
    <div className="space-y-3">
      <h2 className="text-sm font-semibold text-surface-ink">Ziddiyatlar</h2>
      <p className="text-xs leading-relaxed text-surface-soft">
        {report
          ? `${report.contradictions.length} ta ziddiyat topildi.`
          : "Natijalar shu yerda ko‘rinadi."}
      </p>
    </div>
  );
}
