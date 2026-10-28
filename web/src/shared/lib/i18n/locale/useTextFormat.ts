import { useLingui } from "@lingui/react";
import { type TextFormat, textFormat } from "~/shared/lib/text";

/** Активная локаль (`ru`, `en`); компонент перерисуется при её смене. */
export function useLocale(): string {
  return useLingui().i18n.locale;
}

/** Форматтеры чисел, дат и размеров на активной локали; компонент перерисуется при её смене. */
export function useTextFormat(): TextFormat {
  return textFormat(useLocale());
}
