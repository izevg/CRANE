import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

// Юнит-тесты: jsdom, быстрые, по ролям и именам. Подключается проектом `unit` в vitest.config.ts,
// а Stryker запускает его напрямую (мутации не гоняют story-тесты в браузере).
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      name: "unit",
      environment: "jsdom",
      include: ["src/**/*.test.{ts,tsx}"],
      setupFiles: ["./test/setup.ts"],
    },
  }),
);
