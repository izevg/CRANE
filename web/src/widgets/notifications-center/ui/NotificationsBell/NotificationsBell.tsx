import { useLingui } from "@lingui/react";
import { BellIcon } from "lucide-react";
import type * as React from "react";
import { useTextFormat } from "~/shared/lib/i18n";
import { Button } from "~/shared/ui/primitives";
import { bellBadge, bellLabel } from "../../lib/model";

type Props = Omit<
  React.ComponentProps<typeof Button>,
  "size" | "variant" | "children" | "aria-label" | "className"
> & {
  /** Всего непрочитанных (`total` из `GET /notifications/unread-count`). */
  unread: number;
};

/**
 * Кнопка-колокольчик. Остальные пропсы и `ref` уходят на кнопку, поэтому её можно передать
 * в `PopoverTrigger render={…}` как триггер. Число видно значком и читается в имени кнопки.
 */
export function NotificationsBell({ unread, ...props }: Props) {
  const { _ } = useLingui();
  const format = useTextFormat();
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={_(bellLabel(unread))}
      className="relative"
      {...props}
    >
      <BellIcon aria-hidden="true" />
      {unread > 0 ? (
        <span
          aria-hidden="true"
          className="absolute -top-0.5 -right-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-error-bg px-1 text-overline font-semibold text-error tabular-nums ring-1 ring-error-strong"
        >
          {bellBadge(unread, format)}
        </span>
      ) : null}
    </Button>
  );
}
