import { CheckIcon, type LucideIcon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useCallback } from "react";
import { useColorMode } from "~/shared/lib/state";
import { Button, Menu, MenuContent, MenuItem, MenuTrigger } from "~/shared/ui/primitives";

const THEME_LABEL = "Тема";
const SELECTED = ", выбрано";

type ColorMode = ReturnType<typeof useColorMode>["mode"];
type ModeOption = { mode: ColorMode; label: string; Icon: LucideIcon };
type OptionProps = { option: ModeOption; selected: boolean; onSelect: (mode: ColorMode) => void };

const SYSTEM: ModeOption = { mode: "system", label: "Системная", Icon: MonitorIcon };
const MODES: readonly ModeOption[] = [
  { mode: "light", label: "Светлая", Icon: SunIcon },
  { mode: "dark", label: "Тёмная", Icon: MoonIcon },
  SYSTEM,
];

function ThemeOption({ option, selected, onSelect }: OptionProps) {
  const { mode, label, Icon } = option;
  const handleClick = useCallback(() => onSelect(mode), [mode, onSelect]);
  return (
    <MenuItem onClick={handleClick}>
      <Icon aria-hidden="true" />
      <span className="flex-1">{label}</span>
      {selected ? (
        <>
          <CheckIcon aria-hidden="true" />
          <span className="sr-only">{SELECTED}</span>
        </>
      ) : null}
    </MenuItem>
  );
}

/** Переключатель темы: один контрол вмещается и в развёрнутый сайдбар, и в рельсу 64 px. */
export function ThemeMenu({ collapsed }: { collapsed: boolean }) {
  const { mode, setMode } = useColorMode();
  const current = MODES.find((item) => item.mode === mode) ?? SYSTEM;
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
        {MODES.map((option) => (
          <ThemeOption
            key={option.mode}
            option={option}
            selected={option.mode === mode}
            onSelect={setMode}
          />
        ))}
      </MenuContent>
    </Menu>
  );
}
