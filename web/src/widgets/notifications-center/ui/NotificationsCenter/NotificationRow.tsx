import { CircleCheckIcon, InfoIcon, OctagonAlertIcon, TriangleAlertIcon } from "lucide-react";
import { type ComponentType, type SVGProps, useCallback } from "react";
import { cx } from "~/shared/lib/core";
import { Link } from "~/shared/ui/primitives";
import { RelativeTime } from "~/shared/ui/status";
import type { NotificationItem } from "../../lib/model";

const UNREAD_HINT = "Непрочитано. ";

type Glyph = ComponentType<SVGProps<SVGSVGElement>>;

/** Тон различим формой иконки, а не только цветом (WCAG 1.4.1). */
const TONE_ICON = {
  info: InfoIcon,
  success: CircleCheckIcon,
  warning: TriangleAlertIcon,
  error: OctagonAlertIcon,
} satisfies Record<NotificationItem["tone"], Glyph>;

const TONE_COLOR = {
  info: "text-info",
  success: "text-success",
  warning: "text-warning",
  error: "text-error",
} satisfies Record<NotificationItem["tone"], string>;

type Props = {
  item: NotificationItem;
  onOpen: (item: NotificationItem) => void;
};

/** Строка ведёт на объект. Непрочитанная выделена фоном и маркером, а скринридеру об этом говорит текст. */
export function NotificationRow({ item, onOpen }: Props) {
  const handleClick = useCallback(() => onOpen(item), [onOpen, item]);
  const Icon = TONE_ICON[item.tone];
  return (
    <li>
      <Link
        href={item.href}
        onClick={handleClick}
        className={cx(
          "flex min-h-11 items-start gap-2 rounded-md px-2 py-2 text-body hover:bg-accent",
          item.unread && "bg-brand-container",
        )}
      >
        <Icon aria-hidden="true" className={cx("mt-0.5 size-4 shrink-0", TONE_COLOR[item.tone])} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className={cx("break-words", item.unread && "font-semibold")}>
            {item.unread ? <span className="sr-only">{UNREAD_HINT}</span> : null}
            {item.title}
          </span>
          {item.caption === undefined ? null : (
            <span className="truncate text-caption text-muted-foreground">{item.caption}</span>
          )}
        </span>
        <RelativeTime ts={item.time} className="shrink-0 text-caption text-muted-foreground" />
        {item.unread ? (
          <span aria-hidden="true" className="mt-1.5 size-2 shrink-0 rounded-full bg-brand" />
        ) : null}
      </Link>
    </li>
  );
}
