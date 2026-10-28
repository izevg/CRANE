import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { JsonViewer } from "~/shared/ui/data-display";

const COUNT_EN = /^\d+ (keys?|items?)$/;

const meta = {
  title: "Данные/JsonViewer",
  component: JsonViewer,
  args: {
    label: "Метаданные версии",
    value: {
      name: "@acme/ui",
      version: "1.4.2",
      license: "Apache-2.0",
      dist: {
        tarball: "https://crane.acme.example/npm/@acme/ui/-/ui-1.4.2.tgz",
        integrity: "sha512-9fR2kQvXr7LmT0aB5nHcYe1uWdZs8GjP",
        unpackedSize: 188416,
      },
      dependencies: { lodash: "^4.17.21", react: "^19.0.0" },
      keywords: ["ui", "design-system"],
      deprecated: null,
      private: false,
    },
  },
} satisfies Meta<typeof JsonViewer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const VersionMetadata: Story = {
  name: "Метаданные версии",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText('"tarball"')).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Свернуть dist" }));
    await expect(canvas.queryByText('"tarball"')).toBeNull();
    await expect(canvas.getByText("3 ключа")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Развернуть dist" }));
    await expect(canvas.getByText('"tarball"')).toBeVisible();
  },
};

export const CollapsedAll: Story = {
  name: "Свернуть всё",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Свернуть всё" }));
  },
};

export const Dark: Story = { globals: { theme: "dark" } };

export const English: Story = {
  name: "EN: подписи кнопок и счётчики",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole("button", { name: "Expand all" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Collapse all" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Copy JSON" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Collapse all" }));
    await expect(canvas.getAllByText(COUNT_EN)[0]).toBeVisible();
  },
};
