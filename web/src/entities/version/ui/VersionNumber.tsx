import { useLingui } from "@lingui/react/macro";
import type * as React from "react";
import { useId } from "react";
import type { VersionState } from "~/shared/api";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";

type VersionNumberProps = {
  name: string;
  state: VersionState;
};

/** Зачёркнутый номер как триггер подсказки: фокусируется, чтобы подсказка была доступна с клавиатуры. */
function Struck(props: React.ComponentProps<"s">) {
  return <s className="font-mono text-muted-foreground tabular-nums" {...props} />;
}

/**
 * Номер версии: машинное значение, моно. У удалённой он зачёркнут, а подсказка объясняет почему: номер нельзя
 * использовать повторно. Подсказка доступна с клавиатуры.
 */
export function VersionNumber({ name, state }: VersionNumberProps) {
  const { t } = useLingui();
  const hintId = useId();
  if (state !== "deleted") {
    return <span className="font-mono font-semibold tabular-nums">{name}</span>;
  }
  return (
    <>
      <Tooltip>
        <TooltipTrigger render={<Struck tabIndex={0} aria-describedby={hintId} />}>
          {name}
        </TooltipTrigger>
        <TooltipContent>{t`Номер версии нельзя использовать повторно`}</TooltipContent>
      </Tooltip>
      <span id={hintId} className="sr-only">
        {t`Номер версии нельзя использовать повторно`}
      </span>
    </>
  );
}
