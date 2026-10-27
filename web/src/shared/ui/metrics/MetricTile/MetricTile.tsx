import { useLingui } from "@lingui/react/macro";
import { ArrowDownIcon, ArrowUpIcon } from "lucide-react";
import { type ReactNode, useId } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { useTextFormat } from "~/shared/lib/i18n";
import { Card, Link, Skeleton, TONE_SURFACE, toneGlyph } from "~/shared/ui/primitives";
import { RelativeTime } from "~/shared/ui/status";
import { Sparkline } from "../Sparkline/Sparkline";
import {
  type ActiveTone,
  deltaText,
  deltaVerdict,
  formatValue,
  resolveTone,
  type TileDelta,
  type TileTone,
  type TileValue,
  UNIT_GAP,
  valueWithUnit,
} from "./tile";

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

// min-height, а не height: при увеличенных интервалах текста (WCAG 1.4.12) подпись не обрезается.
const HEAD_GRID = "grid grid-cols-[minmax(0,1fr)_auto] items-start gap-x-2";
/** Растянутая ссылка: клик по всей плитке, фокус рисуется на псевдоэлементе по периметру плитки. */
const STRETCHED =
  "outline-none after:absolute after:inset-0 after:rounded-xl after:content-[''] focus-visible:after:outline-2 focus-visible:after:-outline-offset-2 focus-visible:after:outline-ring";
const SIZE = "min-h-28 @3xl/page:min-h-[168px]";
const PADDING = "px-4 py-3 @3xl/page:py-4";

const RING = {
  warning: "ring-1 ring-warning-strong",
  error: "ring-1 ring-error-strong",
} satisfies Record<ActiveTone, string>;

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

const DELTA_STYLE = {
  good: TONE_SURFACE.success,
  bad: TONE_SURFACE.error,
  neutral: TONE_SURFACE.neutral,
} as const;

function DeltaChip({ delta }: { delta: TileDelta }) {
  const { t } = useLingui();
  const format = useTextFormat();
  const Glyph = delta.pct >= 0 ? ArrowUpIcon : ArrowDownIcon;
  const change = deltaText(delta, format);
  return (
    <span
      className={cx(
        "inline-flex h-5 items-center gap-1 rounded-full px-2 text-caption tabular-nums",
        DELTA_STYLE[deltaVerdict(delta)],
      )}
    >
      <Glyph className="size-3" aria-hidden="true" />
      <span className="sr-only">{t`Изменение: ${change}`}</span>
      <span aria-hidden="true">{change}</span>
    </span>
  );
}

function Value({ value, unit }: { value: TileValue; unit: string | undefined }) {
  const format = useTextFormat();
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
        {formatValue(value, format)}
      </span>
      {unit ? (
        <span className="ml-1 text-body text-muted-foreground">
          {UNIT_GAP}
          {unit}
        </span>
      ) : null}
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
  const { t } = useLingui();
  let main: ReactNode = null;
  if (empty) {
    main = <span className="truncate text-caption text-muted-foreground">{t`Нет данных`}</span>;
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
          {t`обновлено`} <RelativeTime ts={stale.updatedAt} />
        </span>
      ) : null}
      {!empty && delta && caption ? <span className="sr-only">{caption}</span> : null}
    </div>
  );
}

type LinkedHead = { href: string; name: string; describedBy: string };

/**
 * Шапка плитки: подпись, значение и тон в сетке (бейдж встаёт вправо вверх, порядок в DOM — как в имени).
 * У плитки-ссылки это и есть `<a>`: его псевдоэлемент растягивается на всю плитку, а подпись, значение и тон —
 * весь видимый текст ссылки, поэтому имя «P0 открыто: 1, требует действия» содержит его целиком (WCAG 2.5.3).
 * Подпись под значением остаётся вне ссылки и читается как её описание.
 */
function Head({ linked, children }: { linked: LinkedHead | null; children: ReactNode }) {
  if (linked === null) {
    return <div className={HEAD_GRID}>{children}</div>;
  }
  return (
    <Link
      href={linked.href}
      aria-label={linked.name}
      aria-describedby={linked.describedBy}
      className={cx(HEAD_GRID, STRETCHED)}
    >
      {children}
    </Link>
  );
}

export function MetricTile(props: MetricTileProps) {
  const { label, value, unit, caption, delta, spark, reference, href, stale, className } = props;
  const { t } = useLingui();
  const format = useTextFormat();
  const labelId = useId();
  const footerId = useId();
  const tone = resolveTone(props);
  const empty = value === null;
  const linked = href !== undefined;
  // Имя плитки-ссылки: «P0 открыто: 1, требует действия». Каждая фраза — целое сообщение: порядок слов зависит от языка.
  const shown = valueWithUnit(value, unit, format) ?? t`нет данных`;
  const toneLabel = tone?.label;
  const linkName = () =>
    toneLabel === undefined ? t`${label}: ${shown}` : t`${label}: ${shown}, ${toneLabel}`;

  const card = (
    <Card
      data-tone={tone?.tone}
      {...(linked ? {} : { role: "group", "aria-labelledby": labelId })}
      className={cx(
        "@container/tile relative gap-0",
        PADDING,
        SIZE,
        tone && RING[tone.tone],
        linked && "hover:shadow-md",
        className,
      )}
    >
      <Head
        linked={
          linked
            ? {
                href,
                name: linkName(),
                describedBy: footerId,
              }
            : null
        }
      >
        <span
          id={labelId}
          title={label}
          className="col-start-1 row-start-1 min-w-0 truncate text-overline text-muted-foreground"
        >
          {label}
        </span>
        <div className="col-span-2 row-start-2 mt-1 flex min-w-0 items-baseline @3xl/page:mt-2">
          <Value value={value} unit={unit} />
        </div>
        {tone ? (
          <div className="col-start-2 row-start-1">
            <ToneBadge tone={tone.tone} label={tone.label} />
          </div>
        ) : null}
      </Head>
      <Footer id={footerId} empty={empty} delta={delta} caption={caption} stale={stale} />
      <div className="mt-auto hidden h-12 @3xl/page:block">
        {spark !== undefined || empty ? (
          <Sparkline values={spark ?? []} reference={reference} tone={tone?.tone ?? "neutral"} />
        ) : null}
      </div>
    </Card>
  );

  return card;
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
