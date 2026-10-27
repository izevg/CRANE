import type { Decorator, Preview } from "@storybook/react-vite";
import "~/shared/styles";
import "../src/app/index.css";
import "./fonts";
import { applyFonts, fontGlobalTypes, fontInitialGlobals } from "./fonts/toolbars";
import { StoryShell } from "./StoryShell";

type ThemeMode = "light" | "dark";
type MotionMode = "system" | "reduce";

/** Тема и reduced motion ставятся на <html>, как в приложении (data-theme) и в CSS кита (data-motion). */
const withGlobals: Decorator = (Story, context) => {
  const { theme = "light", motion = "system" } = context.globals as {
    theme?: ThemeMode;
    motion?: MotionMode;
  };
  const root = document.documentElement;
  root.setAttribute("data-theme", theme);
  root.style.colorScheme = theme;
  applyFonts(context.globals);
  if (motion === "reduce") {
    root.setAttribute("data-motion", "reduce");
  } else {
    root.removeAttribute("data-motion");
  }
  return (
    <StoryShell>
      <Story />
    </StoryShell>
  );
};

const preview: Preview = {
  decorators: [withGlobals],
  globalTypes: {
    ...fontGlobalTypes,
    theme: {
      description: "Тема",
      toolbar: {
        title: "Тема",
        icon: "circlehollow",
        dynamicTitle: true,
        items: [
          { value: "light", title: "Светлая" },
          { value: "dark", title: "Тёмная" },
        ],
      },
    },
    motion: {
      description: "Движение",
      toolbar: {
        title: "Движение",
        icon: "play",
        dynamicTitle: true,
        items: [
          { value: "system", title: "Как в системе" },
          { value: "reduce", title: "Уменьшить (reduced motion)" },
        ],
      },
    },
  },
  initialGlobals: { theme: "light", motion: "system", ...fontInitialGlobals },
  parameters: {
    layout: "padded",
    // Нарушения axe валят прогон story-тестов. Отключать правило можно только в конкретной story и с причиной.
    // context "body": порталы оверлеев (Dialog, Menu, Popover…) живут вне #storybook-root и иначе не попали бы в axe.
    a11y: { test: "error", context: "body" },
  },
};

export default preview;
