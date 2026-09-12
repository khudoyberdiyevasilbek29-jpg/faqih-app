import { AlertTriangle } from "lucide-react";
import type { ContradictionReport } from "../../lib/types";

interface ContradictionReportViewProps {
  report: ContradictionReport;
}

export function ContradictionReportView({ report }: ContradictionReportViewProps) {
  return (
    <div className="space-y-5">
      <section className="panel-card">
        <h3 className="text-sm font-semibold text-surface-ink">Umumiy xulosa</h3>
        <p className="mt-3 text-[15px] leading-relaxed text-surface-soft">
          {report.summary || "Xulosa yo‘q."}
        </p>
        <p className="mt-4 text-xs text-surface-faint">
          Hujjat uzunligi: {report.documentLengthChars.toLocaleString()} belgi
        </p>
      </section>

      {report.wasChunked ? (
        <div className="glass relative flex gap-3 p-5 text-sm text-amber-900 dark:text-amber-100">
          <AlertTriangle
            className="relative z-[1] mt-0.5 h-4 w-4 shrink-0"
            strokeWidth={1.5}
          />
          <p className="relative z-[1] leading-relaxed">
            Hujjat juda uzun bo‘lgani uchun bo‘limlarga bo‘lib tahlil qilindi.
            Bo‘limlar oralig‘idagi ziddiyatlar topilmasligi mumkin — bu ochiq
            cheklov.
          </p>
        </div>
      ) : null}

      {report.contradictions.length === 0 ? (
        <div className="panel-card text-[15px] text-surface-soft">
          Ichki ziddiyatlar topilmadi.
        </div>
      ) : (
        <div className="space-y-4">
          {report.contradictions.map((item, index) => (
            <article
              key={`${index}-${item.statementA.slice(0, 24)}`}
              className="panel-card"
            >
              <p className="text-xs font-medium text-accent">
                Ziddiyat #{index + 1}
              </p>
              <div className="mt-4 grid gap-3 md:grid-cols-2">
                <blockquote className="glass relative p-4 text-[15px] leading-relaxed text-surface-ink">
                  <p className="relative z-[1] mb-2 text-[11px] uppercase tracking-wide text-surface-faint">
                    Bayonot A
                  </p>
                  <span className="relative z-[1]">“{item.statementA}”</span>
                </blockquote>
                <blockquote className="glass relative p-4 text-[15px] leading-relaxed text-surface-ink">
                  <p className="relative z-[1] mb-2 text-[11px] uppercase tracking-wide text-surface-faint">
                    Bayonot B
                  </p>
                  <span className="relative z-[1]">“{item.statementB}”</span>
                </blockquote>
              </div>
              <p className="mt-4 text-[15px] leading-relaxed text-surface-soft">
                {item.explanation}
              </p>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}
