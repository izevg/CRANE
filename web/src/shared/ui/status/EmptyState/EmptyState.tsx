import { useLingui } from "@lingui/react";
import { InboxIcon, type LucideIcon } from "lucide-react";
import { cx } from "~/shared/lib/core";
import { type Action, ActionButton } from "~/shared/ui/actions";
import { FILTERED_TITLE } from "./messages";

type EmptyStateProps = (
  | {
      /** Первый запуск: что здесь появится и с чего начать. */
      variant: "first-run";
      icon?: LucideIcon;
      title: string;
      description?: string;
      primary?: Action;
      secondary?: Action;
    }
  | {
      /** Фильтры ничего не нашли: компактно, без иконки. */
      variant: "filtered";
      title?: string;
      reset?: Action;
    }
) & {
  /** Уровень заголовка: 2 на странице, 3 внутри панели. */
  headingLevel?: 2 | 3;
  /** Только размещение (margin/width/grid) внешнего узла. */
  className?: string;
};

const LAYOUT = "flex flex-col items-center px-6 text-center";
const HEADINGS = { 2: "h2", 3: "h3" } as const;

/**
 * Пустое состояние. Кнопка есть только там, где передано действие: без `Action` вызывающий
 * не получает ни кнопки, ни ссылки.
 */
export function EmptyState(props: EmptyStateProps) {
  const { _ } = useLingui();
  const Heading = HEADINGS[props.headingLevel ?? 2];
  if (props.variant === "filtered") {
    return (
      <div className={cx(LAYOUT, "py-8", props.className)}>
        <Heading className="text-balance text-h3">{props.title ?? _(FILTERED_TITLE)}</Heading>
        {props.reset ? (
          <div className="mt-4">
            <ActionButton action={props.reset} variant="secondary" />
          </div>
        ) : null}
      </div>
    );
  }
  const Icon = props.icon ?? InboxIcon;
  return (
    <div className={cx(LAYOUT, "py-12", props.className)}>
      <span className="flex size-14 items-center justify-center rounded-full bg-muted">
        <Icon className="size-7 text-muted-foreground" aria-hidden="true" />
      </span>
      <Heading className="mt-4 text-balance text-h3">{props.title}</Heading>
      {props.description ? (
        <p className="mt-1 max-w-[48ch] text-body text-muted-foreground">{props.description}</p>
      ) : null}
      {props.primary || props.secondary ? (
        <div className="mt-4 flex flex-wrap justify-center gap-2">
          {props.primary ? <ActionButton action={props.primary} variant="primary" /> : null}
          {props.secondary ? <ActionButton action={props.secondary} variant="secondary" /> : null}
        </div>
      ) : null}
    </div>
  );
}
