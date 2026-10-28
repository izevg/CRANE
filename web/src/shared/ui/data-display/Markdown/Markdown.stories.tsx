import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Markdown } from "~/shared/ui/data-display";

// Ленивый модуль в dev-режиме грузится дольше секунды по умолчанию, особенно при параллельном прогоне.
const LAZY = { timeout: 15_000 };
const BAIT_LINK = /Нажми меня/;
const SAFE_LINK = /Безопасная ссылка/;
const PIXEL_LINK = /pixel/;
const DOCS_LINK = /документации/;

const README = `# @acme/ui

Библиотека интерфейсных компонентов команды **web**.

## Установка

\`\`\`bash
pnpm add @acme/ui@1.4.2
\`\`\`

## Совместимость

Версия 1.4.x поддерживает React 19, ветка 1.3.x — React 18 (только исправления).

Подробности в [документации](https://crane.acme.example/docs/ui) и [журнале изменений](/packages/acme-ui/changelog).

- Темы: светлая и тёмная
- Доступность: WCAG 2.2 AA
`;

const MALICIOUS = `# Недоверенный README

<script>window.hacked = true</script>

<img src=x onerror="window.hacked = true">

[Нажми меня](javascript:alert(1))

[Безопасная ссылка](https://crane.acme.example/docs)
`;

const meta = {
  title: "Данные/Markdown",
  component: Markdown,
  args: { source: README },
} satisfies Meta<typeof Markdown>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Readme: Story = {
  name: "README",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole("heading", { name: "Установка" }, LAZY)).toBeVisible();
    await expect(canvas.getByRole("list")).toBeVisible();
    await expect(canvas.getByRole("link", { name: DOCS_LINK })).toHaveAttribute(
      "href",
      "https://crane.acme.example/docs/ui",
    );
  },
};

export const MaliciousInput: Story = {
  name: "Вредоносный ввод",
  args: { source: MALICIOUS },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("heading", { name: "Недоверенный README" }, LAZY);
    await expect(canvas.queryByRole("link", { name: BAIT_LINK })).toBeNull();
    await expect(canvas.getByRole("link", { name: SAFE_LINK })).toBeVisible();
    await expect(canvasElement.querySelector("script, [onerror]")).toBeNull();
  },
};

export const ReadmeWithImage: Story = {
  name: "README с картинкой",
  args: { source: "# Бейджи\n\n![pixel](https://attacker.example/p.gif)" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("heading", { name: "Бейджи" }, LAZY);
    await expect(canvas.queryByRole("img")).toBeNull();
    await expect(canvasElement.querySelector("img")).toBeNull();
    await expect(canvas.getByRole("link", { name: PIXEL_LINK })).toHaveAttribute(
      "href",
      "https://attacker.example/p.gif",
    );
  },
};

export const Dark: Story = { globals: { theme: "dark" } };
