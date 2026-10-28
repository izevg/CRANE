import { useLingui } from "@lingui/react/macro";
import { Skeleton } from "~/shared/ui/primitives";

/**
 * Ожидание чанка или loader: полоса заголовка и блок содержимого, без спиннера. Показывается только внутри
 * `<main>` (порог 300 мс, минимум 500 мс задаёт роутер), поэтому сайдбар и topbar не перемонтируются.
 */
export function PendingMain() {
  const { t } = useLingui();
  return (
    <div aria-busy="true" className="flex flex-col gap-6">
      <p role="status" className="sr-only">
        {t`Загрузка раздела`}
      </p>
      <Skeleton className="h-9 w-64 max-w-full" />
      <Skeleton className="h-64 w-full" />
    </div>
  );
}
