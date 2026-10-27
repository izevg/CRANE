import { useLingui } from "@lingui/react/macro";
import { lazy, Suspense } from "react";
import { Skeleton } from "../../primitives";

// Разбор markdown и санитайзер тяжёлые: они уезжают в отдельный чанк и грузятся при первом показе.
const MarkdownContent = lazy(() =>
  import("./MarkdownContent").then((module) => ({ default: module.MarkdownContent })),
);

type MarkdownProps = {
  /** Недоверенный markdown (README пакета, текст модели): HTML в нём игнорируется, ссылки санируются. */
  source: string;
};

function MarkdownFallback() {
  const { t } = useLingui();
  return (
    <div role="status" aria-label={t`Загрузка текста`} className="flex flex-col gap-2">
      <Skeleton className="h-4 w-2/3" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-5/6" />
    </div>
  );
}

export function Markdown({ source }: MarkdownProps) {
  return (
    <Suspense fallback={<MarkdownFallback />}>
      <MarkdownContent source={source} />
    </Suspense>
  );
}
