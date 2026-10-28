import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { fitsViewport } from "../frame/fit";
import { desktop, mobile, mobile320, screenParameters } from "../frame/screen";
import { stale } from "./S2Data";
import { S2Package } from "./S2Page";

const meta = {
  title: "Экраны/S2 Карточка пакета",
  component: S2Package,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S2Package>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DesktopLight: Story = {
  name: "Desktop, светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "@acme/ui" })).toBeVisible();
    await expect(canvas.getByRole("region", { name: "Команда установки" })).toHaveTextContent(
      "npm i @acme/ui@1.4.2",
    );
  },
};

export const DesktopDark: Story = {
  name: "Desktop, тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const Mobile: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const ProxyStale: Story = {
  name: "Proxy: stale-баннер",
  args: { proxy: true },
  play: async ({ canvasElement }) => {
    // Текст баннера дублируется в live-регионе (announce), поэтому совпадений может быть два
    const [banner] = await within(canvasElement).findAllByText(stale().text);
    await expect(banner).toBeVisible();
  },
};

export const English: Story = {
  ...DesktopLight,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { name: "Install" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Primary menu" })).toBeVisible();
  },
};

/** Топбар не шире окна на телефоне (WCAG 1.4.10): ширины 390 и 320, обе локали. */
export const Fit390: Story = {
  ...Mobile,
  name: "Телефон 390: топбар помещается",
  play: fitsViewport,
};

export const Fit320: Story = {
  ...Mobile,
  name: "Телефон 320: топбар помещается",
  globals: mobile320,
  play: fitsViewport,
};

export const Fit390English: Story = {
  ...Mobile,
  name: "Телефон 390 EN: топбар помещается",
  globals: { ...mobile, locale: "en" },
  play: fitsViewport,
};

export const Fit320English: Story = {
  ...Mobile,
  name: "Телефон 320 EN: топбар помещается",
  globals: { ...mobile320, locale: "en" },
  play: fitsViewport,
};
