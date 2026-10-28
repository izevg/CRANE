import type { ReactNode } from "react";
import type { WithClassName } from "~/shared/lib/core";

const EMPTY = "—";

type KeyValueItem = {
  label: string;
  /** Значение произвольной разметкой: идентификаторы с копированием (`CopyableId`) передаёт вызывающий. */
  value: ReactNode;
};

type KeyValueListProps = WithClassName<{ items: readonly KeyValueItem[] }>;

function isEmpty(value: ReactNode): boolean {
  return value === null || value === undefined || value === "" || value === false;
}

/**
 * Список «свойство — значение» (`<dl>`): лейбл слева, значение справа; в узком контейнере лейбл над значением.
 * Раскладку определяет ширина самого списка (container query), а не окна.
 */
export function KeyValueList({ items, className }: KeyValueListProps) {
  return (
    <div className={className === undefined ? "@container" : `@container ${className}`}>
      <dl className="m-0 flex flex-col gap-3">
        {items.map((item) => (
          <div
            key={item.label}
            className="flex min-w-0 flex-col gap-0.5 @md:grid @md:grid-cols-[minmax(8rem,12rem)_minmax(0,1fr)] @md:gap-x-4"
          >
            <dt className="text-caption text-muted-foreground">{item.label}</dt>
            <dd className="m-0 min-w-0 text-body tabular-nums wrap-anywhere">
              {isEmpty(item.value) ? (
                <span className="text-muted-foreground">{EMPTY}</span>
              ) : (
                item.value
              )}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
