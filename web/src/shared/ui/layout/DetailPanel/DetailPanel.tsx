import { XIcon } from "lucide-react";
import {
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useCallback,
  useEffect,
  useId,
  useRef,
} from "react";
import { useBreakpoint } from "~/shared/lib/core";
import { type Action, ActionBar, type ActionGroup, ActionMenu } from "../../actions";
import { Button, DialogClose, Sheet, SheetContent, SheetTitle } from "../../primitives";

const CLOSE = "Закрыть";
const HEADING_CLASS = "min-w-0 flex-1 truncate text-h2 outline-none";

type DetailPanelProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Идентификатор или имя сущности: заголовок панели и её доступное имя. */
  title: string;
  status?: ReactNode;
  /** Меню «…» в шапке панели («Открыть полностью» и т. п.). */
  menu?: readonly Action[];
  tabs?: ReactNode;
  children: ReactNode;
  /** Закреплённый подвал: те же правила, что у действий PageHeader. */
  footer?: ActionGroup;
  /** Строка таблицы (или другой элемент), на которую возвращается фокус после закрытия. */
  returnFocusRef?: RefObject<HTMLElement | null>;
};

type PanelBodyProps = Pick<DetailPanelProps, "status" | "menu" | "tabs" | "children" | "footer"> & {
  heading: ReactNode;
  closeButton: ReactNode;
};

/** Шапка, вкладки, прокручиваемое содержимое и подвал: общие для docked-панели и Sheet. */
function PanelBody({ heading, closeButton, status, menu, tabs, children, footer }: PanelBodyProps) {
  return (
    <>
      <div className="flex shrink-0 items-start gap-2 border-b p-4">
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          {heading}
          {status}
        </div>
        {menu ? <ActionMenu actions={menu} /> : null}
        {closeButton}
      </div>
      {tabs ? <div className="shrink-0 px-4">{tabs}</div> : null}
      {/*
        Подвал не перекрывает содержимое, а занимает свою строку flex-колонки: область прокрутки заканчивается
        над ним, поэтому элемент в фокусе не прячется под закреплённой зоной (WCAG 2.4.11).
        scroll-pb даёт ему запас до края.
      */}
      <div className="min-h-0 flex-1 scroll-pb-4 overflow-y-auto p-4">{children}</div>
      {/* empty:hidden — подвал без действий (ActionBar вернул null) не оставляет рамку и отступы */}
      {footer ? (
        <div data-slot="detail-panel-footer" className="shrink-0 border-t bg-card p-4 empty:hidden">
          <ActionBar actions={footer} />
        </div>
      ) : null}
    </>
  );
}

/**
 * Панель сущности справа от таблицы. От 1536 px — docked (немодальный `complementary`, таблица остаётся доступной),
 * ниже — модальный Sheet до 720 px. Фокус при открытии идёт на заголовок, при закрытии возвращается на `returnFocusRef`.
 */
export function DetailPanel({
  open,
  onOpenChange,
  title,
  returnFocusRef,
  ...body
}: DetailPanelProps) {
  const docked = useBreakpoint("2xl");
  const titleId = useId();
  const titleRef = useRef<HTMLHeadingElement>(null);
  const wasOpen = useRef<boolean>(false);

  // Sheet двигает фокус сам (initialFocus/finalFocus); вручную это нужно только docked-панели.
  useEffect(() => {
    if (!docked) {
      return;
    }
    if (open) {
      wasOpen.current = true;
      titleRef.current?.focus({ preventScroll: true });
    } else if (wasOpen.current) {
      wasOpen.current = false;
      returnFocusRef?.current?.focus();
    }
  }, [docked, open, returnFocusRef]);

  const close = useCallback(() => onOpenChange(false), [onOpenChange]);
  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLElement>) => {
      // Меню «…» живёт в портале, но его события всплывают по дереву React: берём только Esc изнутри панели.
      if (
        event.key === "Escape" &&
        !event.defaultPrevented &&
        event.currentTarget.contains(event.target as Node)
      ) {
        close();
      }
    },
    [close],
  );

  if (docked) {
    if (!open) {
      return null;
    }
    return (
      <aside
        aria-labelledby={titleId}
        onKeyDown={handleKeyDown}
        className="sticky top-(--topbar-height) flex max-h-[calc(100svh-var(--topbar-height))] w-[560px] shrink-0 flex-col rounded-xl bg-card shadow-card"
      >
        <PanelBody
          {...body}
          heading={
            <h2 ref={titleRef} id={titleId} tabIndex={-1} className={HEADING_CLASS}>
              {title}
            </h2>
          }
          closeButton={
            <Button variant="ghost" size="icon" aria-label={CLOSE} onClick={close}>
              <XIcon aria-hidden="true" />
            </Button>
          }
        />
      </aside>
    );
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="right"
        showCloseButton={false}
        initialFocus={titleRef}
        finalFocus={returnFocusRef ?? true}
        className="gap-0 data-[side=right]:w-full data-[side=right]:sm:max-w-[720px]"
      >
        <PanelBody
          {...body}
          heading={
            <SheetTitle ref={titleRef} tabIndex={-1} className={HEADING_CLASS}>
              {title}
            </SheetTitle>
          }
          closeButton={
            <DialogClose render={<Button variant="ghost" size="icon" aria-label={CLOSE} />}>
              <XIcon aria-hidden="true" />
            </DialogClose>
          }
        />
      </SheetContent>
    </Sheet>
  );
}
