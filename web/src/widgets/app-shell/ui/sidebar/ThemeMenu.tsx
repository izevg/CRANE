import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { CheckIcon, type LucideIcon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useCallback } from "react";
import { useColorMode } from "~/shared/lib/state";
import { Button, Menu, MenuContent, MenuTrigger } from "~/shared/ui/primitives";

const THEME_LABEL = "Тема";
const MENU_LABEL = "Тема оформления";

type ColorMode = ReturnType<typeof useColorMode>["mode"];
type ModeOption = { mode: ColorMode; label: string; Icon: LucideIcon };

const SYSTEM: ModeOption = { mode: "system", label: "Системная", Icon: MonitorIcon };
const MODES: readonly ModeOption[] = [
  { mode: "light", label: "Светлая", Icon: SunIcon },
  { mode: "dark", label: "Тёмная", Icon: MoonIcon },
  SYSTEM,
];

function isColorMode(value: unknown): value is ColorMode {
  return MODES.some((option) => option.mode === value);
}

// Тот же вид, что у MenuItem кита: в примитивах пока нет radio-пункта (пробел фундамента).
const RADIO_ITEM =
  "relative flex h-9 cursor-default select-none items-center gap-2 rounded-sm px-2 text-body data-highlighted:bg-accent [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

/**
 * Переключатель темы: выбор одного из трёх режимов — `menuitemradio` с `aria-checked`.
 * Один контрол вмещается и в развёрнутый сайдбар, и в рельсу 64 px.
 */
export function ThemeMenu({ collapsed }: { collapsed: boolean }) {
  const { mode, setMode } = useColorMode();
  const current = MODES.find((option) => option.mode === mode) ?? SYSTEM;
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
            aria-label={`${THEME_LABEL}: ${current.label}`}
            className={collapsed ? "w-full px-0" : "w-full justify-start"}
          />
        }
      >
        <current.Icon aria-hidden="true" />
        {collapsed ? null : <span>{`${THEME_LABEL}: ${current.label}`}</span>}
      </MenuTrigger>
      <MenuContent side={collapsed ? "right" : "top"} align="start">
        <MenuPrimitive.RadioGroup value={mode} onValueChange={handleChange} aria-label={MENU_LABEL}>
          {MODES.map(({ mode: value, label, Icon }) => (
            <MenuPrimitive.RadioItem key={value} value={value} className={RADIO_ITEM}>
              <Icon aria-hidden="true" />
              <span className="flex-1">{label}</span>
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
