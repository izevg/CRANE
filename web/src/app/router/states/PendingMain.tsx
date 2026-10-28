import { Skeleton } from "~/shared/ui/primitives";

/**
 * Ожидание чанка или loader: полоса заголовка и блок содержимого, без спиннера. Показывается только внутри
 * `<main>` (порог 300 мс, минимум 500 мс задаёт роутер), поэтому сайдбар и topbar не перемонтируются. Текста для
 * скринридера нет: результат объявит фокус на h1 (монтируемое сообщение читалось бы дважды).
 */
export function PendingMain() {
  return (
    <div aria-busy="true" className="flex flex-col gap-6">
      <Skeleton className="h-9 w-64 max-w-full" />
      <Skeleton className="h-64 w-full" />
    </div>
  );
}
