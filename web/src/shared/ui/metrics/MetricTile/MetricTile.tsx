import { ArrowDownIcon, ArrowUpIcon } from "lucide-react";
import { type ReactNode, useId } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { formatCount, formatNumber, formatPercent } from "~/shared/lib/text";
import { Card, Link, Skeleton, TONE_SURFACE, toneGlyph } from "~/shared/ui/primitives";
import { RelativeTime } from "~/shared/ui/status";
import { Sparkline } from "../Sparkline/Sparkline";

type TileDelta = {
  /** Процентное изменение, например -3.1. */
  pct: number;
  /** Какое направление считается хорошим. */
  polarity: "up-good" | "down-good" | "neutral";
};

/** `toneLabel` обязателен при не нейтральном тоне: цвет один не передаёт статус (WCAG 1.4.1). */
type TileTone = { tone?: "neutral" } | { tone: "warning" | "error"; toneLabel: string };

type TileValue = number | { count: number; capped: boolean } | string | null;

type MetricTileProps = TileTone &
  WithClassName<{
    label: string;
    /** `null` — нет данных: «—» и «Нет данных», никогда не 0. */
    value: TileValue;
    unit?: string;
    caption?: ReactNode;
    delta?: TileDelta;
    spark?: readonly number[];
    /** Линия цели на спарклайне. */
    reference?: number;
    /** Плитка-ссылка: один `Link` на всю плитку. */
    href?: string;
    /** Ошибка опроса: последнее значение остаётся, рядом показывается возраст данных. */
    stale?: { updatedAt: string };
  }>;

const NO_VALUE = "—";
const NO_DATA = "Нет данных";
const NO_DATA_NAME = "нет данных";
const STALE_PREFIX = "обновлено ";
const DELTA_PREFIX = "Изменение: ";
const MINUS = "−";
const NAME_SEPARATOR = ": ";
const TONE_SEPARATOR = ", ";
const PERCENT = 100;

const SIZE = "h-28 @3xl/page:h-[168px]";
const PADDING = "px-4 py-3 @3xl/page:py-4";

type ActiveTone = "warning" | "error";

const RING = {
  warning: "ring-1 ring-warning-strong",
  error: "ring-1 ring-error-strong",
} satisfies Record<ActiveTone, string>;

function resolveTone(props: TileTone): { tone: ActiveTone; label: string } | null {
  if (!("toneLabel" in props)) {
    return null;
  }
  return { tone: props.tone, label: props.toneLabel };
}

/**
 * Бейдж тона. Порог — ширина самой плитки (container query), а не окна: в узкой плитке остаётся иконка,
 * а текст тона лежит в `sr-only`; в широкой виден полный бейдж. Текст один, дублей в DOM нет.
 */
function ToneBadge({ tone, label }: { tone: ActiveTone; label: string }) {
  const Glyph = toneGlyph(tone);
  return (
    <span
      data-tone={tone}
      className={cx(
        "inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full text-caption font-semibold @[15rem]/tile:h-6 @[15rem]/tile:px-2.5",
        TONE_SURFACE[tone],
      )}
    >
      <Glyph aria-hidden="true" className="size-4 shrink-0" />
      <span className="sr-only @[15rem]/tile:not-sr-only">{label}</span>
    </span>
  );
}

function deltaStyle({ pct, polarity }: TileDelta): string {
  if (polarity === "neutral" || pct === 0) {
    return TONE_SURFACE.neutral;
  }
  const good = polarity === "up-good" ? pct > 0 : pct < 0;
  return good ? TONE_SURFACE.success : TONE_SURFACE.error;
}

function DeltaChip({ delta }: { delta: TileDelta }) {
  const Glyph = delta.pct >= 0 ? ArrowUpIcon : ArrowDownIcon;
  const sign = delta.pct < 0 ? MINUS : "";
  return (
    <span
      className={cx(
        "inline-flex h-5 items-center gap-1 rounded-full px-2 text-caption tabular-nums",
        deltaStyle(delta),
      )}
    >
      <Glyph className="size-3" aria-hidden="true" />
      <span className="sr-only">{DELTA_PREFIX}</span>
      {sign}
      {formatPercent(Math.abs(delta.pct) / PERCENT)}
    </span>
  );
}

function formatValue(value: Exclude<TileValue, null>): string {
  if (typeof value === "number") {
    return formatNumber(value);
  }
  return typeof value === "string" ? value : formatCount(value);
}

function Value({ value, unit }: { value: TileValue; unit: string | undefined }) {
  if (value === null) {
    return <span className="text-metric-xl">{NO_VALUE}</span>;
  }
  const text = typeof value === "string";
  return (
    <>
      <span
        className={cx(
          "tabular-nums",
          text ? "min-w-0 truncate whitespace-nowrap text-metric" : "text-metric-xl",
        )}
      >
        {formatValue(value)}
      </span>
      {unit ? <span className="ml-1 text-body text-muted-foreground">{unit}</span> : null}
    </>
  );
}

/** Подпись под значением: нет данных → delta → caption; возраст данных при ошибке опроса — следом. */
function Footer({
  id,
  empty,
  delta,
  caption,
  stale,
}: {
  id: string;
  empty: boolean;
  delta: TileDelta | undefined;
  caption: ReactNode;
  stale: { updatedAt: string } | undefined;
}) {
  let main: ReactNode = null;
  if (empty) {
    main = <span className="truncate text-caption text-muted-foreground">{NO_DATA}</span>;
  } else if (delta) {
    main = <DeltaChip delta={delta} />;
  } else if (caption) {
    main = <span className="truncate text-caption text-muted-foreground">{caption}</span>;
  }
  return (
    <div id={id} className="mt-1 flex h-5 items-center gap-2">
      {main}
      {stale ? (
        <span className="shrink-0 whitespace-nowrap text-caption text-warning">
          {STALE_PREFIX}
          <RelativeTime ts={stale.updatedAt} />
        </span>
      ) : null}
      {!empty && delta && caption ? <span className="sr-only">{caption}</span> : null}
    </div>
  );
}

/** Имя плитки-ссылки: «P0 открыто: 1, требует действия». */
function linkName(label: string, value: TileValue, toneLabel: string | undefined): string {
  const shown = value === null ? NO_DATA_NAME : formatValue(value);
  const name = `${label}${NAME_SEPARATOR}${shown}`;
  return toneLabel === undefined ? name : `${name}${TONE_SEPARATOR}${toneLabel}`;
}

export function MetricTile(props: MetricTileProps) {
  const { label, value, unit, caption, delta, spark, reference, href, stale, className } = props;
  const labelId = useId();
  const footerId = useId();
  const tone = resolveTone(props);
  const empty = value === null;
  const linked = href !== undefined;

  const card = (
    <Card
      data-tone={tone?.tone}
      {...(linked ? {} : { role: "group", "aria-labelledby": labelId })}
      className={cx(
        "@container/tile gap-0",
        PADDING,
        SIZE,
        tone && RING[tone.tone],
        linked ? undefined : className,
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <span
          id={labelId}
          title={label}
          className="min-w-0 flex-1 truncate text-overline text-muted-foreground"
        >
          {label}
        </span>
        {tone ? <ToneBadge tone={tone.tone} label={tone.label} /> : null}
      </div>
      <div className="mt-1 flex min-w-0 items-baseline @3xl/page:mt-2">
        <Value value={value} unit={unit} />
      </div>
      <Footer id={footerId} empty={empty} delta={delta} caption={caption} stale={stale} />
      <div className="mt-auto hidden h-12 @3xl/page:block">
        {spark !== undefined || empty ? (
          <Sparkline values={spark ?? []} reference={reference} tone={tone?.tone ?? "neutral"} />
        ) : null}
      </div>
    </Card>
  );

  if (!linked) {
    return card;
  }
  return (
    <Link
      href={href}
      aria-label={linkName(label, value, tone?.label)}
      aria-describedby={footerId}
      className={cx("block rounded-xl transition-shadow hover:shadow-md", className)}
    >
      {card}
    </Link>
  );
}

/** Скелет той же геометрии, что и плитка. */
export function MetricTileSkeleton({ className }: WithClassName<object>) {
  return (
    <Card aria-busy="true" className={cx("gap-0", PADDING, SIZE, className)}>
      <Skeleton className="h-4 w-24" />
      <Skeleton className="mt-3 h-10 w-28" />
      <Skeleton className="mt-1 h-5 w-16 rounded-full" />
      <Skeleton className="mt-auto hidden h-12 @3xl/page:block" />
    </Card>
  );
}
