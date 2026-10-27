import { useLayoutEffect, useRef } from "react";
import { cx } from "~/shared/lib/core";
import { CopyButton } from "../../actions";

const DEFAULT_LABEL = "Код";

type CodeBlockProps = {
  code: string;
  /** Доступное имя кнопки копирования: «Копировать команду установки». */
  copyLabel: string;
  /** Имя прокручиваемой области кода. */
  label?: string;
  /** Переносить длинные строки; по умолчанию строки прокручиваются по горизонтали. */
  wrap?: boolean;
};

/**
 * Блок кода без подсветки: `--font-mono`, кнопка копирования в углу. Копируется всегда полный текст.
 * Область кода именована (`region`) и попадает в Tab только при переполнении (WCAG 2.1.1), без лишней остановки
 * у короткого текста. Кнопка стоит в разметке перед областью, поэтому получает первый фокус, а не сам текст.
 */
export function CodeBlock({
  code,
  copyLabel,
  label = DEFAULT_LABEL,
  wrap = false,
}: CodeBlockProps) {
  const ref = useRef<HTMLPreElement>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: смена текста не меняет размер рамки, но меняет переполнение
  useLayoutEffect(() => {
    const pre = ref.current;
    if (!pre) {
      return;
    }
    const sync = () => {
      const overflows = pre.scrollWidth > pre.clientWidth || pre.scrollHeight > pre.clientHeight;
      if (overflows) {
        pre.setAttribute("tabindex", "0");
      } else {
        pre.removeAttribute("tabindex");
      }
    };
    sync();
    const observer = new ResizeObserver(sync);
    observer.observe(pre);
    return () => observer.disconnect();
  }, [code, wrap]);
  return (
    <div className="relative">
      <CopyButton
        value={code}
        label={copyLabel}
        className="absolute top-1 right-1 z-[1] bg-(--code-bg)"
      />
      {/* biome-ignore lint/a11y/useSemanticElements: у <pre> нет подходящей роли с именем, нужен именованный регион */}
      <pre
        ref={ref}
        role="region"
        aria-label={label}
        className={cx(
          "max-h-64 overflow-auto rounded-lg bg-(--code-bg) p-3 pr-12 font-mono text-caption text-foreground",
          wrap ? "whitespace-pre-wrap wrap-anywhere" : "whitespace-pre",
        )}
      >
        <code>{code}</code>
      </pre>
    </div>
  );
}
