import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  // Stories лежат рядом с компонентом: Component.stories.tsx
  stories: ["../src/**/*.stories.@(ts|tsx)"],
  addons: ["@storybook/addon-a11y", "@storybook/addon-vitest"],
  // Телеметрия выключена: репозиторий не отправляет данные о работе разработчиков наружу.
  core: { disableTelemetry: true },
  framework: "@storybook/react-vite",
};

export default config;
