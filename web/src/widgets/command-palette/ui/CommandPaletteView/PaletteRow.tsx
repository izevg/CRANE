import { Autocomplete } from "@base-ui/react/autocomplete";
import { ArrowRightIcon, PackageIcon, ZapIcon } from "lucide-react";
import { type ReactNode, useCallback } from "react";
import { cx } from "~/shared/lib/core";
import { Badge } from "~/shared/ui/primitives";
import type { PaletteOption } from "../../lib/palette/model";

const MFA = "MFA";
const STEP_UP_HINT = ", требуется подтверждение входа";

const ITEM =
  "flex min-h-9 cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-body outline-hidden data-highlighted:bg-accent data-disabled:cursor-not-allowed data-disabled:text-muted-foreground [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0";
const ICON = "text-muted-foreground";
const CAPTION = "shrink-0 text-caption text-muted-foreground";
const LABEL = "min-w-0 flex-1 truncate";

function Body({ option }: { option: PaletteOption }): ReactNode {
  if (option.kind === "navigation") {
    const { label, hint } = option.entry;
    return (
      <>
        <ArrowRightIcon aria-hidden="true" className={ICON} />
        <span className={LABEL}>{label}</span>
        {hint === undefined ? null : <span className={CAPTION}>{hint}</span>}
      </>
    );
  }
  if (option.kind === "package") {
    const { name, format, latest } = option.entry;
    return (
      <>
        <PackageIcon aria-hidden="true" className={ICON} />
        <span className={LABEL}>{name}</span>
        <Badge variant="outline">{format}</Badge>
        {latest === undefined ? null : (
          <span className={cx(CAPTION, "tabular-nums")}>{latest}</span>
        )}
      </>
    );
  }
  const { label, stepUp, disabledReason } = option.entry;
  return (
    <>
      <ZapIcon aria-hidden="true" className={ICON} />
      <span className={LABEL}>
        {label}
        {disabledReason === undefined ? null : (
          <span className="block text-caption text-muted-foreground">{disabledReason}</span>
        )}
      </span>
      {stepUp ? (
        <>
          <Badge variant="outline">{MFA}</Badge>
          <span className="sr-only">{STEP_UP_HINT}</span>
        </>
      ) : null}
    </>
  );
}

type Props = {
  option: PaletteOption;
  onPick: (option: PaletteOption) => void;
};

/**
 * Пункт палитры. Недоступное действие (`disabledReason`) — отключённый пункт: Base UI не даёт его выбрать,
 * причина остаётся текстом внутри пункта и читается как часть имени.
 */
export function PaletteRow({ option, onPick }: Props) {
  const blocked = option.kind === "action" && option.entry.disabledReason !== undefined;
  const handleClick = useCallback(() => onPick(option), [onPick, option]);
  const destructive = option.kind === "action" && option.entry.destructive === true;
  return (
    <Autocomplete.Item
      value={option}
      disabled={blocked}
      onClick={handleClick}
      className={cx(ITEM, destructive && "text-error")}
    >
      <Body option={option} />
    </Autocomplete.Item>
  );
}
