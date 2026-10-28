import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import {
  CloudAlertIcon,
  FileQuestionMarkIcon,
  KeyRoundIcon,
  type LucideIcon,
  ShieldAlertIcon,
  WifiOffIcon,
} from "lucide-react";
import { type ReactNode, useId } from "react";
import { cx } from "~/shared/lib/core";
import { type Action, ActionBar, type ActionGroup } from "~/shared/ui/actions";
import { ContactLine, RemediationList, RequestIdLine, RuleLine } from "./ProblemParts";

/**
 * Разобранная проблема. Разбор problem+json в эту модель — забота слоя API: компонент получает готовое.
 * Размеченное объединение делает MUST «404 не выдаёт существование ресурса» непредставимым к нарушению:
 * у `not-found` нет полей для правила, контакта, remediation и «Запросить доступ».
 */
export type ProblemView =
  | { kind: "not-found"; requestId?: string; search?: ReactNode; home: Action }
  | {
      kind: "forbidden";
      ruleId: string;
      requestId: string;
      remediation: readonly string[];
      contact?: string;
      requestAccess?: Action;
      home?: Action;
    }
  | {
      kind: "error";
      /** Смысл ошибки без HTTP-кода: код живёт в диагностике. */
      title: string;
      detail?: string;
      ruleId?: string;
      requestId: string;
      remediation: readonly string[];
      diagnostics?: Action;
      retry?: Action;
    }
  | { kind: "offline"; retry?: Action }
  | { kind: "session"; signIn: Action };

type ProblemStateProps = {
  problem: ProblemView;
  /** Уровень заголовка: 1 для страницы, 2 и 3 внутри панелей. */
  headingLevel?: 1 | 2 | 3;
  /** Плотный вариант для панелей. */
  compact?: boolean;
  /** Только размещение (margin/width/grid) внешнего узла. */
  className?: string;
};

type Head = {
  Icon: LucideIcon;
  circle: string;
  iconTone: string;
  /** Дескриптор — локальный словарь кита; строка — серверный текст, как пришёл. */
  title: MessageDescriptor | string;
  hint?: MessageDescriptor | string | undefined;
};

// Заголовки по kind: серверный `title` для них не показывается (язык сервера не зависит от языка интерфейса).
const KIND_TEXT = {
  "not-found": {
    title: msg`Не найдено или нет доступа`,
    hint: msg`Проверьте адрес или имя пакета`,
  },
  forbidden: { title: msg`Недостаточно прав` },
  offline: {
    title: msg`Нет соединения`,
    hint: msg`Проверьте сеть — данные обновятся, когда связь вернётся`,
  },
  session: {
    title: msg`Сессия завершена`,
    hint: msg`Войдите снова, чтобы продолжить работу`,
  },
} satisfies Record<
  Exclude<ProblemView["kind"], "error">,
  { title: MessageDescriptor; hint?: MessageDescriptor }
>;

const HEADINGS = { 1: "h1", 2: "h2", 3: "h3" } as const;

const NEUTRAL = { circle: "bg-muted", iconTone: "text-muted-foreground" } as const;

function headOf(problem: ProblemView): Head {
  switch (problem.kind) {
    case "not-found":
      return {
        ...NEUTRAL,
        Icon: FileQuestionMarkIcon,
        ...KIND_TEXT["not-found"],
      };
    case "forbidden":
      return {
        Icon: ShieldAlertIcon,
        circle: "bg-warning-bg",
        iconTone: "text-warning",
        ...KIND_TEXT.forbidden,
      };
    case "error":
      return {
        Icon: CloudAlertIcon,
        circle: "bg-error-bg",
        iconTone: "text-error",
        title: problem.title,
        hint: problem.detail,
      };
    case "offline":
      return { ...NEUTRAL, Icon: WifiOffIcon, ...KIND_TEXT.offline };
    case "session":
      return {
        Icon: KeyRoundIcon,
        circle: "bg-info-bg",
        iconTone: "text-info",
        ...KIND_TEXT.session,
      };
  }
}

/**
 * Группа действий проблемы: главное (если есть) и до двух вторичных. Отсутствующий `Action` просто пропускается,
 * поэтому кнопки без действия не бывает (правило HAL).
 */
function problemActions(
  primary: Action | undefined,
  ...secondary: readonly (Action | undefined)[]
): ActionGroup {
  const [first, second] = secondary.filter((action): action is Action => action !== undefined);
  const rest =
    first === undefined
      ? undefined
      : second === undefined
        ? ([first] as const)
        : ([first, second] as const);
  return {
    ...(primary ? { primary } : {}),
    ...(rest ? { secondary: rest } : {}),
  };
}

/** Действия по центру блока; без `Action` не рисуется ничего. */
function ProblemActions({
  primary,
  secondary,
}: {
  primary: Action | undefined;
  secondary?: Action | undefined;
}) {
  return <ActionBar actions={problemActions(primary, secondary)} className="mt-4 justify-center" />;
}

type Of<K extends ProblemView["kind"]> = Extract<ProblemView, { kind: K }>;

function NotFoundDetails({ problem }: { problem: Of<"not-found"> }) {
  return (
    <>
      {problem.search ? <div className="mt-4 w-full max-w-sm">{problem.search}</div> : null}
      {problem.requestId ? <RequestIdLine requestId={problem.requestId} /> : null}
      <ProblemActions primary={problem.home} />
    </>
  );
}

function ForbiddenDetails({ problem }: { problem: Of<"forbidden"> }) {
  return (
    <>
      <RuleLine ruleId={problem.ruleId} />
      <RequestIdLine requestId={problem.requestId} />
      {problem.contact ? <ContactLine contact={problem.contact} /> : null}
      <RemediationList steps={problem.remediation} />
      <ProblemActions primary={problem.requestAccess} secondary={problem.home} />
    </>
  );
}

function ErrorDetails({ problem }: { problem: Of<"error"> }) {
  return (
    <>
      {problem.ruleId ? <RuleLine ruleId={problem.ruleId} /> : null}
      <RequestIdLine requestId={problem.requestId} />
      <RemediationList steps={problem.remediation} />
      <ProblemActions primary={problem.retry} secondary={problem.diagnostics} />
    </>
  );
}

function Details({ problem }: { problem: ProblemView }) {
  switch (problem.kind) {
    case "not-found":
      return <NotFoundDetails problem={problem} />;
    case "forbidden":
      return <ForbiddenDetails problem={problem} />;
    case "error":
      return <ErrorDetails problem={problem} />;
    case "offline":
      return <ProblemActions primary={problem.retry} />;
    case "session":
      return <ProblemActions primary={problem.signIn} />;
  }
}

/**
 * Единый рендер проблемных состояний. Машинный `reason` сюда не попадает: сырой код — только в диагностике.
 * Блок не live-регион: объявление для скринридера (например, ошибка внутри списка) — забота вызывающего через `announce()`.
 */
export function ProblemState({
  problem,
  headingLevel = 2,
  compact = false,
  className,
}: ProblemStateProps) {
  const { t } = useLingui();
  const headingId = useId();
  const head = headOf(problem);
  const Heading = HEADINGS[headingLevel];
  const { Icon } = head;
  return (
    <section
      aria-labelledby={headingId}
      className={cx(
        "flex flex-col items-center px-6 text-center",
        compact ? "py-8" : "py-12",
        className,
      )}
    >
      <span
        className={cx(
          "flex items-center justify-center rounded-full",
          head.circle,
          compact ? "size-10" : "size-14",
        )}
      >
        <Icon className={cx(head.iconTone, compact ? "size-5" : "size-7")} aria-hidden="true" />
      </span>
      <Heading id={headingId} className="mt-4 max-w-[48ch] text-balance text-h3 wrap-anywhere">
        {typeof head.title === "string" ? head.title : t(head.title)}
      </Heading>
      {head.hint ? (
        <p className="mt-1 max-w-[48ch] text-body text-muted-foreground wrap-anywhere">
          {typeof head.hint === "string" ? head.hint : t(head.hint)}
        </p>
      ) : null}
      <Details problem={problem} />
    </section>
  );
}
