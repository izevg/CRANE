import { useLingui } from "@lingui/react/macro";
import { InfoIcon } from "lucide-react";
import type { VersionFlags as VersionFlagsDto } from "~/shared/api";
import {
  Badge,
  Button,
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverTitle,
  PopoverTrigger,
} from "~/shared/ui/primitives";
import { flagItems } from "../lib/flags";

type VersionFlagsProps = {
  flags: VersionFlagsDto;
};

/**
 * Флаг с пояснением автора. Бейдж остаётся текстом, а пояснение открывает отдельная кнопка «ⓘ» (toggletip):
 * наведение и фокус на касании не работают, нажатие работает везде, а кнопка имеет имя для скринридера.
 */
function HintedFlag({ label, hint }: { label: string; hint: string }) {
  const { t } = useLingui();
  return (
    <span className="inline-flex items-center gap-0.5">
      <Badge variant="outline">{label}</Badge>
      <Popover>
        <PopoverTrigger
          render={<Button variant="ghost" size="icon-sm" aria-label={t`Подробнее: ${label}`} />}
        >
          <InfoIcon aria-hidden="true" />
        </PopoverTrigger>
        <PopoverContent>
          <PopoverTitle>{label}</PopoverTitle>
          <PopoverDescription>{hint}</PopoverDescription>
        </PopoverContent>
      </Popover>
    </span>
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
