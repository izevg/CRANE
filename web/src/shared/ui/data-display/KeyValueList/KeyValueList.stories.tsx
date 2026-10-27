import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { CopyableId } from "~/shared/ui/actions";
import { KeyValueList } from "~/shared/ui/data-display";
import { Badge } from "~/shared/ui/primitives";

const PURL = "pkg:npm/%40acme/ui@1.4.2";
const SHA = "a88d4f0e91c7b2236f5d0c8a7e4b19d3c5f6a2810b7e94d3c1f58a2b6e1d6973";

const meta = {
  title: "Данные/KeyValueList",
  component: KeyValueList,
  args: {
    items: [
      { label: "Формат", value: <Badge variant="outline">npm</Badge> },
      { label: "PURL", value: <CopyableId plain value={PURL} full label="Скопировать PURL" /> },
      { label: "SHA-256", value: <CopyableId value={SHA} label="Скопировать SHA-256" /> },
      { label: "Размер", value: "184 КБ" },
      { label: "Лицензия", value: "Apache-2.0" },
      { label: "Описание", value: null },
    ],
  },
} satisfies Meta<typeof KeyValueList>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PackageMetadata: Story = {
  name: "Метаданные пакета",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("term")).toHaveLength(6);
    await expect(canvas.getByText("Apache-2.0")).toBeVisible();
    await expect(canvas.getAllByRole("definition")[5]).toHaveTextContent("—");
  },
};

export const Narrow: Story = {
  name: "Узкий контейнер: лейбл над значением",
  decorators: [
    (Story) => (
      <div className="w-[280px]">
        <Story />
      </div>
    ),
  ],
};

export const Dark: Story = { globals: { theme: "dark" } };
