import { LockIcon } from "lucide-react";
import { type ReactNode, useId } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";

type ReadonlyValueProps = WithClassName<{
  /** Подпись значения; это не `<label>`: контрола в паре нет, привязывать подпись не к чему. */
  label: ReactNode;
  value: ReactNode;
  /** Значение задано окружением и здесь не меняется: значок замка и подсказка, почему и что делать. */
  lockHint?: ReactNode;
  /** Моноширинный шрифт: ключи, переменные, URL. */
  mono?: boolean;
}>;

/**
 * Заблокированное значение с подписью как текст, а не `disabled`-инпут: его можно выделить, скопировать и прочитать
 * скринридером. Подпись и значение связаны через `fieldset` с `aria-labelledby`, поэтому `FieldLabel` рядом не нужен
 * (у него был бы `for` на несуществующий контрол). Текст — `--foreground` на `--surface-high` (контраст 10.6);
 * `--hint` на этой поверхности запрещён (4.35), рамку инпута на ней не рисуем (3:1 не набирается). Подсказка замка
 * лежит на `--card` и несёт смысл текстом, а не только серым цветом.
 */
export function ReadonlyValue({
  label,
  value,
  lockHint,
  mono = false,
  className,
}: ReadonlyValueProps) {
  const labelId = useId();
  const hintId = useId();
  const locked = lockHint !== undefined;
  return (
    <div className={cx("flex min-w-0 flex-col gap-1.5", className)}>
      <span id={labelId} className="text-body font-semibold">
        {label}
      </span>
      {/* fieldset — единственный нативный элемент-группа, который именуется через aria-labelledby без role */}
      <fieldset
        aria-labelledby={labelId}
        aria-describedby={locked ? hintId : undefined}
        className="m-0 flex min-w-0 flex-col gap-1 border-0 p-0"
      >
        <div className="flex min-h-9 items-start gap-2 rounded-md bg-surface-high px-3 py-2 text-body text-foreground">
          {locked ? (
            <LockIcon aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          ) : null}
          <span className={cx("min-w-0 wrap-anywhere", mono && "font-mono")}>{value}</span>
        </div>
        {locked ? (
          <p id={hintId} className="m-0 text-caption text-muted-foreground">
            {lockHint}
          </p>
        ) : null}
      </fieldset>
    </div>
  );
}
