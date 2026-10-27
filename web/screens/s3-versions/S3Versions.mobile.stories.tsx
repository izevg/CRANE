import type { Meta, StoryObj } from "@storybook/react-vite";
import { fitsViewport } from "../frame/fit";
import { desktop, mobile, mobile320, screenParameters } from "../frame/screen";
import { S3Screen } from "./S3Page";

const meta = {
  title: "Экраны/S3 Версии",
  component: S3Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S3Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Mobile390: Story = {
  name: "Mobile 390 · список",
  args: { initialPanelOpen: false },
  globals: mobile,
};

export const Mobile390Panel: Story = {
  name: "Mobile 390 · панель",
  globals: mobile,
};

/** Топбар не шире окна на телефоне (WCAG 1.4.10): ширины 390 и 320, обе локали. */
export const Fit390: Story = {
  ...Mobile390,
  name: "Телефон 390: топбар помещается",
  play: fitsViewport,
};

export const Fit320: Story = {
  ...Mobile390,
  name: "Телефон 320: топбар помещается",
  globals: mobile320,
  play: fitsViewport,
};

export const Fit390English: Story = {
  ...Mobile390,
  name: "Телефон 390 EN: топбар помещается",
  globals: { ...mobile, locale: "en" },
  play: fitsViewport,
};

export const Fit320English: Story = {
  ...Mobile390,
  name: "Телефон 320 EN: топбар помещается",
  globals: { ...mobile320, locale: "en" },
  play: fitsViewport,
};
