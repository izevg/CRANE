import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { CheckIcon, type LucideIcon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useCallback } from "react";
import { COLOR_MODES, type ColorMode, isColorMode, useColorMode } from "~/shared/lib/state";
import { Button, Menu, MenuContent, MenuTrigger } from "~/shared/ui/primitives";

const ICONS = {
  light: SunIcon,
  dark: MoonIcon,
  system: MonitorIcon,
} satisfies Record<ColorMode, LucideIcon>;

const LABELS = {
  light: msg`Светлая`,
  dark: msg`Тёмная`,
  system: msg`Системная`,
} satisfies Record<ColorMode, MessageDescriptor>;

// Тот же вид, что у MenuItem кита: в примитивах пока нет radio-пункта (пробел фундамента).
const RADIO_ITEM =
  "relative flex h-9 cursor-default select-none items-center gap-2 rounded-sm px-2 text-body data-highlighted:bg-accent [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

/**
 * Переключатель темы: выбор одного из трёх режимов — `menuitemradio` с `aria-checked`.
 * Один контрол вмещается и в развёрнутый сайдбар, и в рельсу 64 px.
 */
export function ThemeMenu({ collapsed }: { collapsed: boolean }) {
  const { t } = useLingui();
  const { mode, setMode } = useColorMode();
  const CurrentIcon = ICONS[mode];
  const modeLabel = t(LABELS[mode]);
  const triggerLabel = t`Тема: ${modeLabel}`;
  const handleChange = useCallback(
    (value: unknown) => {
      if (isColorMode(value)) {
        setMode(value);
      }
    },
    [setMode],
  );
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
        <CurrentIcon aria-hidden="true" />
        {collapsed ? null : <span>{triggerLabel}</span>}
      </MenuTrigger>
      <MenuContent side={collapsed ? "right" : "top"} align="start">
        <MenuPrimitive.RadioGroup
          value={mode}
          onValueChange={handleChange}
          aria-label={t`Тема оформления`}
        >
          {COLOR_MODES.map((value) => {
            const Icon = ICONS[value];
            return (
              <MenuPrimitive.RadioItem key={value} value={value} className={RADIO_ITEM}>
                <Icon aria-hidden="true" />
                <span className="flex-1">{t(LABELS[value])}</span>
                <MenuPrimitive.RadioItemIndicator>
                  <CheckIcon aria-hidden="true" />
                </MenuPrimitive.RadioItemIndicator>
              </MenuPrimitive.RadioItem>
            );
          })}
        </MenuPrimitive.RadioGroup>
      </MenuContent>
    </Menu>
  );
}
