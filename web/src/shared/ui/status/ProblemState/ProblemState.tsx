import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import {
  CalendarClockIcon,
  CloudAlertIcon,
  FileQuestionMarkIcon,
  KeyRoundIcon,
  type LucideIcon,
  RefreshCwIcon,
  ServerOffIcon,
  ShieldAlertIcon,
  WifiOffIcon,
} from "lucide-react";
import { type ReactNode, useId } from "react";
import { cx } from "~/shared/lib/core";
import { i18n } from "~/shared/lib/i18n";
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
      /** Нет у ответа, который сервер не снабдил идентификатором (например, страница прокси). */
      requestId?: string;
      remediation: readonly string[];
      diagnostics?: Action;
      retry?: Action;
    }
  | {
      /** Сбой самого интерфейса, не API: у него нет `request_id`, а текст не зависит от сервера. */
      kind: "client";
      /** `chunk` — не загрузился код раздела (после выкладки новой версии); `render` — исключение при рендере. */
      cause: "chunk" | "render";
      reload?: Action;
    }
  | {
      kind: "offline";
      /** `network` — у пользователя нет сети (по умолчанию); `server` — сеть есть, сервер CRANE не отвечает. Обе причины повторяются автоматически (делает вызывающий). */
      cause?: "network" | "server";
      retry?: Action;
    }
  | ({
      kind: "session";
      /** Причина из 401 сессии; учитывается, только когда есть действие входа. */
      reason?: SessionReason;
    } & (
      | { signIn: Action; availableIn?: undefined }
      /**
       * Входа ещё нет: кнопка не рисуется, текст называет версию, в которой он появится. Версию передаёт вызывающий
       * (пометку о замене ставит слой API: в кит её поставить нельзя).
       */
      | { signIn?: undefined; availableIn: string }
    ))
  /** Операция ещё заглушена: данных нет, раздел (или вся веб-консоль, `scope: "console"`) появится в версии. */
  | { kind: "planned"; replacedIn: string; scope?: "section" | "console" };

export type SessionReason = "session_expired" | "session_revoked" | "session_elsewhere";

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
} satisfies Record<
  "not-found" | "forbidden",
  { title: MessageDescriptor; hint?: MessageDescriptor }
>;

const CLIENT_TEXT = {
  chunk: {
    title: msg`Не удалось загрузить раздел`,
    hint: msg`Возможно, CRANE обновился. Обновите страницу.`,
  },
  render: {
    title: msg`Не удалось открыть страницу`,
    hint: msg`Обновите страницу. Если ошибка повторится, сообщите администратору CRANE.`,
  },
} satisfies Record<"chunk" | "render", { title: MessageDescriptor; hint: MessageDescriptor }>;

const OFFLINE_TEXT = {
  network: {
    title: msg`Нет соединения`,
    hint: msg`Проверьте сеть — данные обновятся, когда связь вернётся.`,
  },
  server: {
    title: msg`Сервер CRANE не отвечает`,
    hint: msg`Повторим попытку автоматически. Если сервер долго недоступен, сообщите администратору CRANE.`,
  },
} satisfies Record<"network" | "server", { title: MessageDescriptor; hint: MessageDescriptor }>;

const SESSION_ENDED = msg`Сессия завершена`;

/** Причина 401: первое предложение подсказки; второе — действие («Войдите снова…»). */
const SESSION_REASON = {
  session_expired: msg`Время сессии истекло.`,
  session_revoked: msg`Сессия отозвана.`,
  session_elsewhere: msg`Вы вошли на другом устройстве, и эта сессия завершена.`,
} satisfies Record<SessionReason, MessageDescriptor>;

const HEADINGS = { 1: "h1", 2: "h2", 3: "h3" } as const;

const NEUTRAL = { circle: "bg-muted", iconTone: "text-muted-foreground" } as const;

/** Версия для текста: «0.4.0», без префикса «v». */
function bare(version: string): string {
  return version.startsWith("v") ? version.slice(1) : version;
}

function sessionHead(problem: Of<"session">): Head {
  const base = { Icon: KeyRoundIcon, circle: "bg-info-bg", iconTone: "text-info" } as const;
  if (problem.signIn === undefined) {
    // Входа нет: причину не показываем, она ничего не даёт пользователю, который войти не может.
    const version = bare(problem.availableIn);
    return {
      ...base,
      title: msg`Войдите, чтобы продолжить`,
      hint: msg`Вход в веб-консоль появится в CRANE ${version}.`,
    };
  }
  if (problem.reason === undefined) {
    return {
      ...base,
      title: msg`Войдите, чтобы продолжить`,
      hint: msg`Для работы с реестром нужна сессия.`,
    };
  }
  return {
    ...base,
    title: SESSION_ENDED,
    hint: `${i18n._(SESSION_REASON[problem.reason])} ${i18n._(msg`Войдите снова, чтобы продолжить работу.`)}`,
  };
}

function plannedHead(problem: Of<"planned">): Head {
  const version = bare(problem.replacedIn);
  return {
    ...NEUTRAL,
    Icon: CalendarClockIcon,
    ...(problem.scope === "console"
      ? {
          title: msg`Веб-консоль появится в CRANE ${version}`,
          hint: msg`Сейчас она недоступна.`,
        }
      : {
          title: msg`Раздел пока недоступен`,
          hint: msg`Он появится в CRANE ${version}.`,
        }),
  };
}

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
    case "client":
      return {
        Icon: RefreshCwIcon,
        circle: "bg-error-bg",
        iconTone: "text-error",
        ...CLIENT_TEXT[problem.cause],
      };
    case "offline": {
      const cause = problem.cause ?? "network";
      return {
        ...NEUTRAL,
        Icon: cause === "server" ? ServerOffIcon : WifiOffIcon,
        ...OFFLINE_TEXT[cause],
      };
    }
    case "session":
      return sessionHead(problem);
    case "planned":
      return plannedHead(problem);
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
      {problem.requestId ? <RequestIdLine requestId={problem.requestId} /> : null}
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
    case "client":
      return <ProblemActions primary={problem.reload} />;
    case "offline":
      return <ProblemActions primary={problem.retry} />;
    case "session":
      return <ProblemActions primary={problem.signIn} />;
    case "planned":
      return null;
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
