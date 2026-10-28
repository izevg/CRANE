import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { ReactNode } from "react";
import { useEffect } from "react";
import { announce } from "~/shared/lib/a11y";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Callout, Skeleton, TONE_TEXT, type Tone, toneGlyph } from "../../primitives";
import { type ProbeStatus, summarize } from "./summary";

export type ProbeCheck = {
  id: string;
  label: string;
  status: ProbeStatus;
  /** Пояснение сервера как есть: не переводится и не интерпретируется. */
  detail?: string;
};

type ProbeReportState =
  | "idle"
  | "running"
  | { checks: readonly ProbeCheck[] }
  | { error: ReactNode };

type ProbeReportProps = WithClassName<{
  state: ProbeReportState;
  /** Сколько проверок будет в отчёте: на время `running` под текстом резервируется столько строк-заглушек. */
  expected?: number;
}>;

const STATUS_TONE = {
  pass: "success",
  warn: "warning",
  fail: "error",
  skipped: "neutral",
} satisfies Record<ProbeStatus, Tone>;

/**
 * Строка отчёта (py-1.5 + строка текста) = `h-8`. Пока отчёта нет, место под одну строку занято; под несколько —
 * только если вызывающий сказал `expected`. Строки с `detail` выше заглушки, поэтому сдвиг возможен и тогда.
 */
const RESERVE = "min-h-8";
const ROW_SKELETON = "h-8";

function skeletonSlots(count: number): string[] {
  return Array.from({ length: count }, (_, slot) => `slot-${slot}`);
}

function useStatusLabels(): Record<ProbeStatus, string> {
  const { t } = useLingui();
  return {
    pass: t`успешно`,
    warn: t`предупреждение`,
    fail: t`ошибка`,
    skipped: t`пропущено`,
  };
}

function ProbeRow({ check, status }: { check: ProbeCheck; status: string }) {
  const tone = STATUS_TONE[check.status];
  const Icon = toneGlyph(tone);
  return (
    <li className="flex items-start gap-2 py-1.5">
      <Icon
        aria-hidden="true"
        className={cx("mt-0.5 size-4 shrink-0 forced-colors:text-[CanvasText]", TONE_TEXT[tone])}
      />
      <div className="min-w-0">
        <p className="m-0 text-body wrap-anywhere">
          <span>{check.label}</span>
          {" · "}
          <span className={cx("font-semibold", TONE_TEXT[tone])}>{status}</span>
        </p>
        {check.detail === undefined ? null : (
          <p className="m-0 text-caption text-muted-foreground wrap-anywhere">{check.detail}</p>
        )}
      </div>
    </li>
  );
}

function ProbeResult({ checks }: { checks: readonly ProbeCheck[] }) {
  const { t } = useLingui();
  const labels = useStatusLabels();
  const { pass, warn, fail } = summarize(checks);
  const summary = t`Проверка завершена: ${pass} успешно, ${plural(warn, {
    one: "# с предупреждением",
    few: "# с предупреждениями",
    many: "# с предупреждениями",
    other: "# с предупреждениями",
  })}, ${plural(fail, {
    one: "# с ошибкой",
    few: "# с ошибками",
    many: "# с ошибками",
    other: "# с ошибками",
  })}.`;
  // Итог объявляется один раз при появлении отчёта; строки скринридеру не зачитываются
  useEffect(() => {
    announce(summary);
  }, [summary]);
  return (
    <div>
      <p className="m-0 text-caption text-muted-foreground">{summary}</p>
      <ul className="m-0 list-none p-0">
        {checks.map((check) => (
          <ProbeRow key={check.id} check={check} status={labels[check.status]} />
        ))}
      </ul>
    </div>
  );
}

/**
 * Отчёт проверки связи: строки со статусом (значок + текст + тон), итог одним сообщением.
 * Место под отчёт резервируется заранее, поэтому появление строк не двигает форму.
 */
export function ProbeReport({ state, expected = 0, className }: ProbeReportProps) {
  const { t } = useLingui();
  let content: ReactNode = null;
  if (state === "running") {
    content = (
      <>
        <p aria-busy="true" className="m-0 py-1.5 text-body text-muted-foreground">
          {t`Проверяем связь…`}
        </p>
        {skeletonSlots(expected).map((slot) => (
          <Skeleton key={slot} className={ROW_SKELETON} />
        ))}
      </>
    );
  } else if (typeof state === "object" && "checks" in state) {
    content = <ProbeResult checks={state.checks} />;
  } else if (typeof state === "object") {
    content = (
      <Callout tone="error" live="alert">
        {state.error}
      </Callout>
    );
  }
  return <div className={cx(RESERVE, className)}>{content}</div>;
}
