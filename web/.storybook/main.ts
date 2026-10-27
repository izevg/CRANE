import type { StorybookConfig } from "@storybook/react-vite";
import { FS_ALLOW } from "../vite.config.ts";

const config: StorybookConfig = {
  // Stories лежат рядом с компонентом: Component.stories.tsx
  stories: ["../src/**/*.stories.@(ts|tsx)"],
  addons: ["@storybook/addon-a11y", "@storybook/addon-vitest"],
  // Телеметрия выключена: репозиторий не отправляет данные о работе разработчиков наружу.
  core: { disableTelemetry: true },
  framework: "@storybook/react-vite",
  // Storybook сам задаёт server.fs.allow (только корень worktree) и не берёт значение из vite.config.ts; без ссылок
  // на реальный node_modules основного каталога dev-сервер в worktree Orca отвечает 403 на файлы зависимостей.
  viteFinal: (config) => ({
    ...config,
    server: {
      ...config.server,
      fs: { ...config.server?.fs, allow: [...(config.server?.fs?.allow ?? []), ...FS_ALLOW] },
    },
  }),
};

export default config;
