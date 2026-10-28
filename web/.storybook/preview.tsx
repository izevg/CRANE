import type { Decorator, Preview } from "@storybook/react-vite";
import "~/shared/styles";
import "../src/app/entry/index.css";
import { applyOverrides } from "~/shared/api/stubs/override/override";
import { activateLocale, i18n, type Locale } from "~/shared/lib/i18n";
import { ApiStory } from "./ApiStory";
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

/**
 * Клиент запросов свежий на историю (`key` по id). Подмены API из `parameters.api` (operationId → обработчик)
 * ставит загрузчик истории (`loaders`), а не рендер: сброс и установка идут один раз перед каждой историей. Состояния «загрузка» и «ошибка» задаются подменой, а не моками сети.
 */
const withApi: Decorator = (Story, context) => {
  return (
    <ApiStory key={context.id}>
      <Story />
    </ApiStory>
  );
};

const preview: Preview = {
  // Каталог грузится до рендера story; выбор в хранилище витрина не пишет (activateLocale без записи).
  loaders: [
    ({ parameters }) => {
      const { api } = parameters;
      applyOverrides(api);
    },
    async ({ globals }) => {
      const locale = (globals as { locale?: Locale }).locale ?? "ru";
      await activateLocale(locale);
      // Сообщения экранов витрины ложатся поверх каталога приложения и в продовый бандл не попадают.
      const screens =
        locale === "en"
          ? await import("../screens/locales/en.po")
          : await import("../screens/locales/ru.po");
      i18n.load(locale, screens.messages);
    },
  ],
  decorators: [withApi, withGlobals],
  globalTypes: {
    locale: {
      description: "Язык интерфейса",
      toolbar: {
        title: "Язык",
        icon: "globe",
        dynamicTitle: true,
        items: [
          { value: "ru", title: "RU" },
          { value: "en", title: "EN" },
        ],
      },
    },
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
  initialGlobals: { theme: "light", motion: "system", locale: "ru" },
  parameters: {
    layout: "padded",
    // Нарушения axe валят прогон story-тестов. Отключать правило можно только в конкретной story и с причиной.
    // context "body": порталы оверлеев (Dialog, Menu, Popover…) живут вне #storybook-root и иначе не попали бы в axe.
    a11y: { test: "error", context: "body" },
  },
};

export default preview;
