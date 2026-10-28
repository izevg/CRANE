import { useLingui } from "@lingui/react/macro";
import { Check, Copy } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { copyToClipboard, cx } from "~/shared/lib/core";
import { showToast } from "~/shared/lib/state";
import { Button } from "../../primitives";

const COPIED_MS = 1200;

type CopyButtonProps = {
  value: string;
  /** Обязательное доступное имя, например «Скопировать GUID». */
  label: string;
  className?: string;
};

const ICON =
  "absolute inset-0 transition-[opacity,transform] duration-(--dur-fast) ease-(--ease-out) motion-safe:will-change-transform";

/** Копирует значение; «Скопировано» объявляется через общий live-регион, иконка на секунду меняется на галочку. */
export function CopyButton({ value, label, className }: CopyButtonProps) {
  const { t } = useLingui();
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  const copy = useCallback(async () => {
    if (!(await copyToClipboard(value))) {
      showToast.error({ title: t`Не удалось скопировать — выделите текст вручную` });
      return;
    }
    setCopied(true);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setCopied(false), COPIED_MS);
  }, [value, t]);

  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={label}
      data-state={copied ? "copied" : "idle"}
      {...(className === undefined ? {} : { className })}
      onClick={copy}
    >
      <span className="relative size-4">
        <Copy
          aria-hidden="true"
          className={cx(ICON, copied ? "opacity-0 motion-safe:scale-[0.8]" : "opacity-100")}
        />
        <Check
          aria-hidden="true"
          className={cx(
            ICON,
            "text-success",
            copied ? "opacity-100" : "opacity-0 motion-safe:scale-[0.8]",
          )}
        />
      </span>
    </Button>
  );
}
