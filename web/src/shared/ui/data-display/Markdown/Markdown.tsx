import { useLingui } from "@lingui/react";
import { lazy, Suspense } from "react";
import { Skeleton } from "../../primitives";
import { MARKDOWN_LOADING } from "../text/messages";

// Разбор markdown и санитайзер тяжёлые: они уезжают в отдельный чанк и грузятся при первом показе.
const MarkdownContent = lazy(() =>
  import("./MarkdownContent").then((module) => ({ default: module.MarkdownContent })),
);

type MarkdownProps = {
  /** Недоверенный markdown (README пакета, текст модели): HTML в нём игнорируется, ссылки санируются. */
  source: string;
};

function MarkdownFallback() {
  const { _ } = useLingui();
  return (
    <div role="status" aria-label={_(MARKDOWN_LOADING)} className="flex flex-col gap-2">
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
