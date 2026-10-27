import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { useLingui } from "@lingui/react/macro";
import { CheckIcon, GlobeIcon } from "lucide-react";
import { useCallback } from "react";
import { isLocale, LOCALES, type Locale, setLocale, useLocale } from "~/shared/lib/i18n";
import { Button, Menu, MenuContent, MenuTrigger } from "~/shared/ui/primitives";

// Название языка пишется на нём самом и не переводится: человек должен найти свой язык при любом выбранном.
const ENDONYMS = {
  // i18n-ignore: эндоним языка
  ru: "Русский",
  en: "English",
} satisfies Record<Locale, string>;

// Тот же вид, что у MenuItem кита: в примитивах пока нет radio-пункта (пробел фундамента).
const RADIO_ITEM =
  "relative flex h-9 cursor-default select-none items-center gap-2 rounded-sm px-2 text-body data-highlighted:bg-accent [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

/**
 * Переключатель языка интерфейса: `menuitemradio` с `aria-checked`, как у `ThemeMenu`. Выбор сохраняется
 * (`crane.locale`) и меняет `<html lang>`.
 */
export function LocaleMenu({ collapsed }: { collapsed: boolean }) {
  const { t } = useLingui();
  const locale = useLocale();
  const current = isLocale(locale) ? ENDONYMS[locale] : locale;
  const triggerLabel = t`Язык: ${current}`;
  const handleChange = useCallback((value: unknown) => {
    if (isLocale(value)) {
      void setLocale(value);
    }
  }, []);
  return (
    <Menu>
      <MenuTrigger
        render={
          <Button
            variant="ghost"
            aria-label={triggerLabel}
            className={collapsed ? "w-full px-0" : "w-full justify-start"}
          />
        }
      >
        <GlobeIcon aria-hidden="true" />
        {collapsed ? null : <span>{triggerLabel}</span>}
      </MenuTrigger>
      <MenuContent side={collapsed ? "right" : "top"} align="start">
        <MenuPrimitive.RadioGroup
          value={locale}
          onValueChange={handleChange}
          aria-label={t`Язык интерфейса`}
        >
          {LOCALES.map((value) => (
            <MenuPrimitive.RadioItem key={value} value={value} className={RADIO_ITEM}>
              <span className="flex-1">{ENDONYMS[value]}</span>
              <MenuPrimitive.RadioItemIndicator>
                <CheckIcon aria-hidden="true" />
              </MenuPrimitive.RadioItemIndicator>
            </MenuPrimitive.RadioItem>
          ))}
        </MenuPrimitive.RadioGroup>
      </MenuContent>
    </Menu>
  );
}
