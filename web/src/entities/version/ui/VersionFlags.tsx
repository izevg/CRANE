import { useId } from "react";
import type { VersionFlags as VersionFlagsDto } from "~/shared/api";
import { Badge, Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";
import { flagItems } from "../lib/flags";

type VersionFlagsProps = {
  flags: VersionFlagsDto;
};

/** Флаг с пояснением: подсказка для мыши и фокуса, то же пояснение — доступное описание триггера. */
function HintedFlag({ label, hint }: { label: string; hint: string }) {
  const hintId = useId();
  return (
    <>
      <Tooltip>
        <TooltipTrigger render={<Badge variant="outline" tabIndex={0} aria-describedby={hintId} />}>
          {label}
        </TooltipTrigger>
        <TooltipContent>{hint}</TooltipContent>
      </Tooltip>
      <span id={hintId} className="sr-only">
        {hint}
      </span>
    </>
  );
}

/** Флаги формата отдельной колонкой: outline-бейдж на флаг. Нет флагов — нет и контейнера. */
export function VersionFlags({ flags }: VersionFlagsProps) {
  const items = flagItems(flags);
  if (items.length === 0) {
    return null;
  }
  return (
    <span className="flex flex-wrap gap-1">
      {items.map((item) =>
        item.hint === undefined ? (
          <Badge key={item.id} variant="outline">
            {item.label}
          </Badge>
        ) : (
          <HintedFlag key={item.id} label={item.label} hint={item.hint} />
        ),
      )}
    </span>
  );
}
