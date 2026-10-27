import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useState } from "react";
import { type Action, ActionButton } from "~/shared/ui/actions";
import {
  Button,
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
  Skeleton,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "~/shared/ui/primitives";
import { type NotificationCategory, type NotificationItem, unreadPhrase } from "../../lib/model";
import { ALL_TAB, isCategory, tabCount, tabOf } from "../../lib/tabs";
import { NotificationsBell } from "../NotificationsBell/NotificationsBell";
import { NotificationRow } from "./NotificationRow";

const TABS = [
  { value: ALL_TAB, label: msg`Все` },
  { value: "security", label: msg`Безопасность` },
  { value: "access", label: msg`Доступ` },
  { value: "platform", label: msg`Платформа` },
] as const;

const SKELETON_ROWS = ["first", "second", "third"] as const;

type Props = {
  items: readonly NotificationItem[];
  /** Всего непрочитанных: число на колокольчике и во вкладке «Все». */
  unread: number;
  /** Непрочитанные по категориям (`by_category`) для счётчиков вкладок. */
  unreadByCategory: Readonly<Record<NotificationCategory, number>>;
  /** Текущая вкладка; `undefined` — «Все». Данные под неё подгружает вызывающий. */
  category?: NotificationCategory;
  onCategoryChange: (category?: NotificationCategory) => void;
  loading?: boolean;
  /** Нет действия — нет кнопки «Загрузить ещё». */
  loadMore?: Action;
  /** Вызывается при переходе по строке (`crane:mark-read`). */
  onMarkRead?: (id: string) => void;
  /** Нет обработчика — нет кнопки «Прочитать все». Получает текущую категорию. */
  onReadAll?: (category?: NotificationCategory) => void;
};

function Rows({
  items,
  onOpen,
}: {
  items: readonly NotificationItem[];
  onOpen: (item: NotificationItem) => void;
}) {
  return (
    <ul className="flex flex-col gap-0.5">
      {items.map((item) => (
        <NotificationRow key={item.id} item={item} onOpen={onOpen} />
      ))}
    </ul>
  );
}

function Skeletons() {
  return (
    <div aria-hidden="true" className="flex flex-col gap-2 px-2">
      {SKELETON_ROWS.map((row) => (
        <Skeleton key={row} className="h-9 w-full" />
      ))}
    </div>
  );
}

/**
 * Центр уведомлений (K2): колокольчик и Popover 360 px. Данные, опрос и пометка прочитанным — снаружи;
 * виджет только показывает список и сообщает о действиях пользователя.
 */
export function NotificationsCenter({
  items,
  unread,
  unreadByCategory,
  category,
  onCategoryChange,
  loading = false,
  loadMore,
  onMarkRead,
  onReadAll,
}: Props) {
  const { t } = useLingui();
  const [open, setOpen] = useState(false);
  const active = tabOf(category);

  const handleTab = useCallback(
    (value: unknown) => {
      onCategoryChange(isCategory(value) ? value : undefined);
    },
    [onCategoryChange],
  );

  const handleOpen = useCallback(
    (item: NotificationItem) => {
      onMarkRead?.(item.id);
      setOpen(false);
    },
    [onMarkRead],
  );

  const handleReadAll = useCallback(() => onReadAll?.(category), [onReadAll, category]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger render={<NotificationsBell unread={unread} />} />
      <PopoverContent align="end" size="wide" flush className="max-w-[calc(100vw-2rem)]">
        <div className="flex items-center justify-between gap-2 px-3 pt-3">
          <PopoverTitle>{t`Уведомления`}</PopoverTitle>
          {onReadAll === undefined ? null : (
            <Button variant="ghost" onClick={handleReadAll}>
              {t`Прочитать все`}
            </Button>
          )}
        </div>
        <Tabs value={active} onValueChange={handleTab}>
          <TabsList variant="line" className="mx-3">
            {TABS.map((tab) => {
              const count = tabCount(tab.value, unread, unreadByCategory);
              return (
                <TabsTrigger key={tab.value} value={tab.value}>
                  {t(tab.label)}
                  {count > 0 ? (
                    <>
                      <span aria-hidden="true" className="text-caption tabular-nums">
                        {count}
                      </span>
                      <span className="sr-only">{`, ${t(unreadPhrase(count))}`}</span>
                    </>
                  ) : null}
                </TabsTrigger>
              );
            })}
          </TabsList>
          {TABS.map((tab) => (
            <TabsContent
              key={tab.value}
              value={tab.value}
              className="max-h-[min(24rem,60vh)] overflow-y-auto px-1 pb-2"
            >
              {tab.value === active ? (
                <PanelBody
                  items={items}
                  loading={loading}
                  loadMore={loadMore}
                  onOpen={handleOpen}
                />
              ) : null}
            </TabsContent>
          ))}
        </Tabs>
      </PopoverContent>
    </Popover>
  );
}

function PanelBody({
  items,
  loading,
  loadMore,
  onOpen,
}: {
  items: readonly NotificationItem[];
  loading: boolean;
  loadMore: Action | undefined;
  onOpen: (item: NotificationItem) => void;
}) {
  const { t } = useLingui();
  if (loading) {
    return <Skeletons />;
  }
  if (items.length === 0) {
    return (
      <p className="px-3 py-6 text-center text-body text-muted-foreground">{t`Новых уведомлений нет`}</p>
    );
  }
  return (
    <>
      <Rows items={items} onOpen={onOpen} />
      {loadMore === undefined ? null : (
        <div className="flex justify-center pt-2">
          <ActionButton action={loadMore} variant="ghost" />
        </div>
      )}
    </>
  );
}
