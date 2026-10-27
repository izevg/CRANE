import { useLingui } from "@lingui/react";
import { useLayoutEffect, useRef } from "react";
import { cx } from "~/shared/lib/core";
import { CopyButton } from "../../actions";
import { MachineValue } from "../../primitives";
import { CODE_LABEL } from "../text/messages";

type CodeBlockProps = {
  code: string;
  /** Доступное имя кнопки копирования: «Копировать команду установки». */
  copyLabel: string;
  /** Имя прокручиваемой области кода. */
  label?: string;
  /** Переносить длинные строки; по умолчанию строки прокручиваются по горизонтали. */
  wrap?: boolean;
  /** Секрет, хэш или токен: символы красятся по классам (до 64 знаков). Для команд и конфигов не включать. */
  machine?: boolean;
};

/**
 * Блок кода без подсветки: `--font-mono`, кнопка копирования в углу. Копируется всегда полный текст.
 * Область кода именована (`region`) и попадает в Tab только при переполнении (WCAG 2.1.1), без лишней остановки
 * у короткого текста. Кнопка стоит в разметке перед областью, поэтому получает первый фокус, а не сам текст.
 */
export function CodeBlock({
  code,
  copyLabel,
  label,
  wrap = false,
  machine = false,
}: CodeBlockProps) {
  const { _ } = useLingui();
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
  // Кнопка стоит рядом с областью кода, а не поверх неё: прямоугольники не пересекаются по построению,
  // длинная строка прокручивается внутри своей области. В разметке кнопка идёт первой (первый фокус в диалоге),
  // визуально справа (`order-last`).
  return (
    <div className="flex items-start gap-2 rounded-lg bg-(--code-bg) p-2">
      <span className="order-last shrink-0 rounded-md border border-border bg-card">
        <CopyButton value={code} label={copyLabel} />
      </span>
      {/* biome-ignore lint/a11y/useSemanticElements: у <pre> нет подходящей роли с именем, нужен именованный регион */}
      <pre
        ref={ref}
        role="region"
        aria-label={label ?? _(CODE_LABEL)}
        className={cx(
          "m-0 max-h-64 min-w-0 flex-1 overflow-auto p-1 font-mono text-caption text-foreground",
          wrap ? "whitespace-pre-wrap wrap-anywhere" : "whitespace-pre",
        )}
      >
        <code>
          <MachineValue value={code} plain={!machine} />
        </code>
      </pre>
    </div>
  );
}
