import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { desktop, screenParameters } from "../frame/screen";
import { expectScreen, wideParameters } from "./S3Frames";
import { S3Screen } from "./S3Page";

const meta = {
  title: "Экраны/S3 Версии",
  component: S3Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S3Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

/** 1440: панель открывается поверх таблицы как Sheet (правило кита: docked только от 1536). */
export const Desktop1440Sheet: Story = {
  name: "Desktop 1440 · Sheet",
  play: async ({ canvasElement }) => {
    // Модальный Sheet делает страницу за ним инертной: проверяем саму панель
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Версия 1.4.2" })).toBeVisible();
    // Версия — машинное значение: моно JetBrains, а не шрифт текста
    const table = canvasElement.querySelector("table");
    const version = within(table as HTMLElement).getAllByText("1.4.1")[0];
    await expect(getComputedStyle(version as HTMLElement).fontFamily).toContain("JetBrains Mono");
  },
};

/** 1600: панель docked справа от таблицы, колонки приоритета 3 скрыты. */
export const Desktop1600Docked: Story = {
  name: "Desktop 1600 · панель docked",
  parameters: wideParameters,
  globals: { viewport: { value: "wide", isRotated: false } },
  play: async ({ canvasElement }) => {
    await expectScreen(canvasElement);
  },
};

export const DesktopDark: Story = {
  name: "Desktop dark · docked",
  parameters: wideParameters,
  globals: { viewport: { value: "wide", isRotated: false }, theme: "dark" },
};

export const Desktop1440Dark: Story = {
  name: "Desktop 1440 dark · Sheet",
  globals: { ...desktop, theme: "dark" },
};

export const English: Story = {
  ...Desktop1440Sheet,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Version 1.4.2" })).toBeVisible();
    // Подписи состояний — словарь интерфейса: в EN таблица и панель одного языка
    const table = canvasElement.querySelector("table");
    await expect(table?.textContent).toContain("Quarantine");
    await expect(table?.textContent).not.toContain("Карантин");
    await expect(table?.textContent).not.toContain("Выпущена");
  },
};
