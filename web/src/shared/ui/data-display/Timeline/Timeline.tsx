import type { ReactNode } from "react";
import { cx } from "~/shared/lib/core";
import { type Action, ActionButton } from "../../actions";
import { Badge } from "../../primitives";
import type { StatusTone } from "../../status";

/**
 * Цвет точки дублируется тегом: если тон не нейтральный, `tag` обязателен (WCAG 1.4.1).
 */
type ToneProps =
  | { tone?: "neutral"; tag?: string }
  | { tone: Exclude<StatusTone, "neutral">; tag: string };

type TimelineItem = {
  id: string;
  time: ReactNode;
  title: string;
  caption?: string;
  /** Ссылка на связанный объект («#4521», CVE). */
  link?: Action;
} & ToneProps;

type TimelineProps = {
  /** Порядок задаёт вызывающий: новые сверху в истории, хронологический в жизненном цикле. */
  items: readonly TimelineItem[];
  /** Доступное имя списка: «История приоритета». */
  label: string;
};

const DOT = {
  success: "bg-success-strong",
  info: "bg-info-strong",
  warning: "bg-warning-strong",
  error: "bg-error-strong",
  neutral: "bg-muted-foreground",
} satisfies Record<StatusTone, string>;

/** Вертикальная лента событий: упорядоченный список `<ol>`. */
export function Timeline({ items, label }: TimelineProps) {
  return (
    <ol aria-label={label} className="m-0 flex list-none flex-col p-0">
      {items.map((item) => (
        <li key={item.id} className="group/item relative flex gap-3 pb-4 last:pb-0">
          <div aria-hidden="true" className="relative flex w-3 shrink-0 justify-center">
            <span className="absolute top-4 -bottom-4 w-px bg-border group-last/item:hidden" />
            <span
              className={cx(
                "relative mt-1.5 size-2.5 rounded-full ring-4 ring-card forced-color-adjust-none",
                DOT[item.tone ?? "neutral"],
              )}
            />
          </div>
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <div className="flex flex-wrap items-baseline gap-x-2 text-caption text-muted-foreground">
              {item.time}
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-body font-semibold wrap-anywhere">{item.title}</span>
              {item.tag === undefined ? null : <Badge variant="outline">{item.tag}</Badge>}
            </div>
            {item.caption === undefined ? null : (
              <p className="m-0 text-caption text-muted-foreground wrap-anywhere">{item.caption}</p>
            )}
            {item.link ? (
              <div>
                <ActionButton action={item.link} variant="ghost" />
              </div>
            ) : null}
          </div>
        </li>
      ))}
    </ol>
  );
}
