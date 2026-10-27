import { type ReactNode, useCallback } from "react";
import { cx } from "~/shared/lib/core";
import { useTextFormat } from "~/shared/lib/i18n";
import { type Action, ActionButton } from "../../actions";
import { Badge, Link, TONE_STRONG, type Tone } from "../../primitives";
import { StatusBadge } from "../../status";

type Status = { tone: Tone; label: string };

/**
 * Цвет точки дублируется текстом: если тон точки не нейтральный, обязателен `status` или `source` (WCAG 1.4.1).
 */
/** `source` — что вызвало изменение («по правилу», «фид»): отдельный чип, не смешивается со статусом. */
type DotTone =
  | { tone?: "neutral"; status?: Status; source?: string }
  | { tone: Exclude<Tone, "neutral">; status: Status; source?: string }
  | { tone: Exclude<Tone, "neutral">; status?: Status; source: string };

type TimelineItem = {
  id: string;
  /** Момент события (ISO-строка, число или `Date`); выводится в `<time>` в локали интерфейса. */
  time: string | number | Date;
  title: string;
  /** Кто изменил: человек, робот (`crr_9f8e7d6c · ui-release`). */
  actor?: string;
  /** Деталь под заголовком: текст или `CopyableId` (хэш, id). */
  detail?: ReactNode;
  /** Ссылка на связанный объект («#4521», CVE): текстовая, левый край совпадает с заголовком. */
  link?: Action;
} & DotTone;

type TimelineProps = {
  /** Порядок задаёт вызывающий: новые сверху в истории, хронологический в жизненном цикле. */
  items: readonly TimelineItem[];
  /** Доступное имя списка: «История приоритета». */
  label: string;
};

const SEPARATOR = "·";
const NBSP = " ";

// Классы записаны целиком: Tailwind находит их по тексту исходников.
// Порог узкого режима (20rem) считается по ширине самой ленты (`@container/timeline` на `<ol>`).
const ROW_START = "col-start-2 col-span-2 @max-[20rem]/timeline:col-span-1";
// StatusBadge выравнивает флекс-элементы по центру, и его базовая линия берётся от иконки, а не от текста (на 3px выше).
// Чип центрируется на первой строке заголовка (24px против строки 20px → смещение −2px): разница базовых линий ≤ 1px.
const STATUS_CELL =
  "col-start-3 row-start-1 -mt-0.5 self-start @max-[20rem]/timeline:col-start-2 @max-[20rem]/timeline:row-start-2 @max-[20rem]/timeline:mt-0 @max-[20rem]/timeline:justify-self-start";
const LINK =
  "inline-flex min-h-6 items-center text-body text-brand hover:underline focus-visible:underline";

function isoOf(time: TimelineItem["time"]): string | undefined {
  const ms = time instanceof Date ? time.getTime() : new Date(time).getTime();
  return Number.isNaN(ms) ? undefined : new Date(ms).toISOString();
}

/** Текстовая ссылка вместо кнопки; действие с причиной недоступности или step-up остаётся кнопкой кита. */
function TimelineLink({ action }: { action: Action }) {
  const { run } = action;
  const handleClick = useCallback(() => {
    if ("onSelect" in run) {
      run.onSelect();
    }
  }, [run]);
  if (action.disabledReason !== undefined || action.stepUp || action.destructive) {
    return <ActionButton action={action} variant="ghost" />;
  }
  if ("href" in run) {
    return (
      <Link href={run.href} className={LINK}>
        {action.label}
      </Link>
    );
  }
  return (
    <button type="button" onClick={handleClick} className={LINK}>
      {action.label}
    </button>
  );
}

function Meta({ item }: { item: TimelineItem }) {
  const format = useTextFormat();
  const segments: { key: string; node: ReactNode }[] = [
    {
      key: "time",
      node: (
        <time dateTime={isoOf(item.time)} title={format.dateTime(item.time)}>
          {format.dateTimeShort(item.time)}
        </time>
      ),
    },
  ];
  if (item.actor !== undefined) {
    segments.push({ key: "actor", node: <span>{item.actor}</span> });
  }
  if (item.source !== undefined) {
    segments.push({ key: "source", node: <Badge variant="outline">{item.source}</Badge> });
  }
  return (
    <div
      className={cx(
        ROW_START,
        "flex flex-wrap items-center gap-x-1 text-caption text-muted-foreground tabular-nums",
      )}
    >
      {segments.map(({ key, node }, index) => (
        <span key={key} className="whitespace-nowrap">
          {index === 0 ? null : (
            <span aria-hidden="true">
              {NBSP}
              {SEPARATOR}{" "}
            </span>
          )}
          {node}
        </span>
      ))}
    </div>
  );
}

/**
 * Вертикальная лента событий: упорядоченный список. Ряд 1: точка, заголовок, статус на одной базовой линии;
 * далее время · актор · источник, деталь и ссылка, все от левого края текста заголовка.
 * Точка вырезается кольцом цвета `--timeline-surface` (по умолчанию `--card`; в Popover и Sheet задаётся `--popover`).
 */
export function Timeline({ items, label }: TimelineProps) {
  return (
    <ol aria-label={label} className="@container/timeline m-0 flex list-none flex-col p-0">
      {items.map((item) => (
        <li
          key={item.id}
          className="group/item relative grid grid-cols-[12px_minmax(0,1fr)_auto] items-baseline gap-x-3 gap-y-1 pb-4 last:pb-0 @max-[20rem]/timeline:grid-cols-[12px_minmax(0,1fr)]"
        >
          <span
            aria-hidden="true"
            className="absolute top-[10px] -bottom-[26px] left-[5.5px] w-px bg-border group-last/item:hidden"
          />
          <span
            aria-hidden="true"
            data-slot="timeline-dot"
            className={cx(
              "relative z-[1] col-start-1 row-start-1 mt-[5px] size-2.5 self-start justify-self-center rounded-full ring-4 ring-[var(--timeline-surface,var(--card))] forced-color-adjust-none",
              TONE_STRONG[item.tone ?? "neutral"],
            )}
          />
          <span className="col-start-2 row-start-1 text-body font-semibold text-pretty wrap-break-word hyphens-auto">
            {item.title}
          </span>
          {item.status ? (
            <span className={STATUS_CELL}>
              <StatusBadge tone={item.status.tone} label={item.status.label} />
            </span>
          ) : null}
          <Meta item={item} />
          {item.detail === undefined ? null : (
            <div className={cx(ROW_START, "text-caption text-foreground")}>{item.detail}</div>
          )}
          {item.link ? (
            <div className={ROW_START}>
              <TimelineLink action={item.link} />
            </div>
          ) : null}
        </li>
      ))}
    </ol>
  );
}
