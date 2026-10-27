import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

/**
 * Зависимости, которые Vite в browser-режиме иначе нашёл бы на лету (ленивые чанки, подпути Base UI):
 * на холодном кэше это перезагружало страницу посреди прогона и роняло сбор story-тестов.
 * Новый подпуть `@base-ui/react/*` или тяжёлую библиотеку добавляйте сюда.
 */
const PREBUNDLED = [
  "@base-ui/react/alert-dialog",
  "@base-ui/react/autocomplete",
  "@base-ui/react/button",
  "@base-ui/react/checkbox",
  "@base-ui/react/dialog",
  "@base-ui/react/field",
  "@base-ui/react/input",
  "@base-ui/react/menu",
  "@base-ui/react/merge-props",
  "@base-ui/react/meter",
  "@base-ui/react/popover",
  "@base-ui/react/select",
  "@base-ui/react/switch",
  "@base-ui/react/tabs",
  "@base-ui/react/toggle",
  "@base-ui/react/toggle-group",
  "@base-ui/react/tooltip",
  "@base-ui/react/use-render",
  "@tanstack/react-table",
  "@tanstack/react-virtual",
  "lucide-react",
  "react-markdown",
  "recharts",
  "rehype-sanitize",
  "sonner",
];

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      projects: [
        "./vitest.unit.config.ts",
        {
          // Story-тесты: каждая story рендерится в настоящем Chromium, играет play-функцию и проходит axe.
          extends: true,
          plugins: [storybookTest({ configDir: ".storybook" })],
          optimizeDeps: { include: PREBUNDLED },
          test: {
            name: "storybook",
            browser: {
              enabled: true,
              headless: true,
              provider: playwright(),
              instances: [{ browser: "chromium" }],
            },
          },
        },
      ],
    },
  }),
);
