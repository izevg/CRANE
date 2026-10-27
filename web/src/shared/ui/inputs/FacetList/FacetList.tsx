import { CheckIcon } from "lucide-react";
import { useCallback, useId } from "react";
import { useTextFormat } from "~/shared/lib/i18n";
import { Card } from "~/shared/ui/primitives";

type FacetItem = {
  value: string;
  label: string;
  count: { count: number; capped: boolean };
};

type FacetListProps = {
  title: string;
  /** Уровень заголовка карточки. */
  headingLevel?: 2 | 3;
  items: readonly FacetItem[];
  selected: readonly string[];
  onToggle: (value: string) => void;
  /** Только размещение (margin/width/grid) внешнего узла. */
  className?: string;
};

const HEADINGS = { 2: "h2", 3: "h3" } as const;
const SEPARATOR = " ";

function maxCount(items: readonly FacetItem[]): number {
  return items.reduce((max, item) => Math.max(max, item.count.count), 0);
}

type FacetRowProps = {
  item: FacetItem;
  pressed: boolean;
  max: number;
  onToggle: (value: string) => void;
};

function FacetRow({ item, pressed, max, onToggle }: FacetRowProps) {
  const format = useTextFormat();
  const handleClick = useCallback(() => onToggle(item.value), [item.value, onToggle]);
  return (
    <li>
      <button
        type="button"
        aria-pressed={pressed}
        onClick={handleClick}
        className="group/facet flex min-h-8 w-full flex-col gap-1 rounded-md px-2 py-1.5 text-left text-body transition-colors duration-(--dur-fast) hover:bg-accent aria-pressed:bg-(--row-selected-bg) aria-pressed:shadow-[inset_2px_0_0_var(--row-selected-bar)] forced-colors:aria-pressed:bg-[Highlight] forced-colors:aria-pressed:text-[HighlightText]"
      >
        <span className="flex w-full items-baseline justify-between gap-2">
          <span className="flex min-w-0 items-baseline gap-1.5">
            {pressed ? (
              <CheckIcon
                data-slot="facet-check"
                aria-hidden="true"
                className="size-4 shrink-0 self-center text-brand"
              />
            ) : null}
            <span className="wrap-anywhere">{item.label}</span>
          </span>
          {SEPARATOR}
          <span className="shrink-0 tabular-nums">{format.count(item.count)}</span>
        </span>
        <span aria-hidden="true" className="h-1 w-full overflow-hidden rounded-full bg-muted">
          <span
            className="block h-full origin-left rounded-full bg-muted-foreground group-aria-pressed/facet:bg-brand forced-colors:bg-[ButtonText] forced-colors:group-aria-pressed/facet:bg-[HighlightText]"
            style={{ transform: `scaleX(${max === 0 ? 0 : item.count.count / max})` }}
          />
        </span>
      </button>
    </li>
  );
}

/**
 * Фасет: строки-кнопки «название · число · доля от максимума». Нажатая строка — активный фильтр
 * (`aria-pressed`), отмечена галочкой и полосой слева, а не только цветом; фон тот же спокойный, что у выбранной строки таблицы (WCAG 1.4.1). Доля нарисована полосой и скрыта от скринридера: число говорит то же самое.
 */
export function FacetList({
  title,
  headingLevel = 2,
  items,
  selected,
  onToggle,
  className,
}: FacetListProps) {
  const headingId = useId();
  if (items.length === 0) {
    return null;
  }
  const Heading = HEADINGS[headingLevel];
  const max = maxCount(items);
  return (
    <Card className={className}>
      <Heading id={headingId} className="px-4 text-h3">
        {title}
      </Heading>
      <ul aria-labelledby={headingId} className="flex flex-col gap-0.5 px-2">
        {items.map((item) => (
          <FacetRow
            key={item.value}
            item={item}
            pressed={selected.includes(item.value)}
            max={max}
            onToggle={onToggle}
          />
        ))}
      </ul>
    </Card>
  );
}
