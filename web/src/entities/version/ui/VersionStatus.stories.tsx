import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { VersionStatus } from "../index";

const meta = {
  title: "Сущности/Версия/Состояние",
  component: VersionStatus,
  args: { version: { state: "released", block_reason: null } },
} satisfies Meta<typeof VersionStatus>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Released: Story = {
  name: "Выпущена",
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("Выпущена")).toBeVisible();
  },
};

export const Quarantined: Story = {
  name: "Карантин",
  args: { version: { state: "quarantined", block_reason: null } },
};

export const Deleted: Story = {
  name: "Удалена",
  args: { version: { state: "deleted", block_reason: null } },
};

/** Остаётся открытым в момент axe-проверки: причина рядом с состоянием, исправленная версия — текстом рядом. */
export const BlockedEmergency: Story = {
  name: "Заблокирована экстренно: причина и исправленная версия рядом",
  args: {
    version: {
      state: "blocked",
      block_reason: {
        kind: "emergency",
        block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
        advisory_id: "CVE-2026-31337",
        fixed_in: "4.17.21",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Заблокирована")).toBeVisible();
    await expect(canvas.getByText("Экстренно · CVE-2026-31337")).toBeVisible();
    await expect(canvas.getByText("Исправлено в 4.17.21")).toBeVisible();
  },
};

export const BlockedByRule: Story = {
  name: "Заблокирована по правилу",
  args: {
    version: {
      state: "blocked",
      block_reason: { kind: "rule", rule_id: "policy.deny.unlicensed" },
    },
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByText("По правилу · policy.deny.unlicensed"),
    ).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: {
    version: {
      state: "blocked",
      block_reason: {
        kind: "emergency",
        block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
        advisory_id: "CVE-2026-31337",
      },
    },
  },
};

export const English: Story = {
  name: "EN: подписи словаря",
  globals: { locale: "en" },
  args: {
    version: {
      state: "blocked",
      block_reason: {
        kind: "emergency",
        block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
        advisory_id: "CVE-2026-31337",
        fixed_in: "4.17.21",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("Blocked")).toBeVisible();
    await expect(canvas.getByText("Emergency · CVE-2026-31337")).toBeVisible();
  },
};
