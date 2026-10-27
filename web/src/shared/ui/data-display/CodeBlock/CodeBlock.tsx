import { cx } from "~/shared/lib/core";
import { CopyButton } from "../../actions";

type CodeBlockProps = {
  code: string;
  /** Доступное имя кнопки копирования: «Копировать команду установки». */
  copyLabel: string;
  /** Переносить длинные строки; по умолчанию строки прокручиваются по горизонтали. */
  wrap?: boolean;
};

/**
 * Блок кода без подсветки: `--font-mono`, кнопка копирования в углу. Копируется всегда полный текст.
 * Область прокрутки именована и фокусируется с клавиатуры (WCAG 2.1.1).
 */
export function CodeBlock({ code, copyLabel, wrap = false }: CodeBlockProps) {
  return (
    <div className="relative">
      <pre
        // biome-ignore lint/a11y/noNoninteractiveTabindex: область с прокруткой доступна с клавиатуры (WCAG 2.1.1)
        tabIndex={0}
        className={cx(
          "max-h-64 overflow-auto rounded-lg bg-(--code-bg) p-3 pr-12 font-mono text-caption text-foreground",
          wrap ? "whitespace-pre-wrap wrap-anywhere" : "whitespace-pre",
        )}
      >
        <code>{code}</code>
      </pre>
      <CopyButton
        value={code}
        label={copyLabel}
        className="absolute top-1 right-1 bg-(--code-bg)"
      />
    </div>
  );
}
