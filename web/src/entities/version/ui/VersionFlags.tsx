import type { VersionFlags as VersionFlagsDto } from "~/shared/api";
import { Badge, Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";
import { flagItems } from "../lib/flags";

type VersionFlagsProps = {
  flags: VersionFlagsDto;
};

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
          <Tooltip key={item.id}>
            <TooltipTrigger render={<Badge variant="outline" tabIndex={0} />}>
              {item.label}
            </TooltipTrigger>
            <TooltipContent>{item.hint}</TooltipContent>
          </Tooltip>
        ),
      )}
    </span>
  );
}
