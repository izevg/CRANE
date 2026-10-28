import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

// Юнит-тесты: jsdom, быстрые, по ролям и именам. Подключается проектом `unit` в vitest.config.ts,
// а Stryker запускает его напрямую (мутации не гоняют story-тесты в браузере).
export default mergeConfig(
  viteConfig,
  defineConfig({
    resolve: {
      // Алиас подменяет @testing-library/react на test/rtl.tsx: render и renderHook оборачивают рендер в I18nProvider
      // (русская локаль активируется в test/setup.ts). Выбран вместо правки 66 импортов и обёртки в каждом тесте:
      // новые тесты получают провайдер сами, а у RTL нет глобального wrapper. Тесты импортируют как обычно.
      alias: [
        {
          find: /^@testing-library\/react$/,
          replacement: new URL("./test/rtl.tsx", import.meta.url).pathname,
        },
      ],
    },
    test: {
      name: "unit",
      // Локальная машина: unit и story-тесты запускаются вместе, jsdom-воркеры не должны занимать все ядра.
      maxWorkers: 4,
      // Первая группа: см. комментарий у проекта storybook в vitest.config.ts.
      sequence: { groupOrder: 0 },
      environment: "jsdom",
      include: ["src/**/*.test.{ts,tsx}"],
      setupFiles: ["./test/setup.ts"],
    },
  }),
);
