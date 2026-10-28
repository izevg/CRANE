import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, spyOn, userEvent, waitFor, within } from "storybook/test";
import { CodeBlock } from "~/shared/ui/data-display";

// Ссылка на переменную окружения собирается из частей, чтобы линтер не принял её за шаблонную строку.
const TOKEN_REF = ["$", "{CRANE_TOKEN}"].join("");
const NPMRC = `@acme:registry=https://crane.acme.example/npm/\n//crane.acme.example/npm/:_authToken=${TOKEN_REF}`;

const meta = {
  title: "Данные/CodeBlock",
  component: CodeBlock,
  // Буфер Chromium в headless недоступен без разрешений: подменяем запись, проверяем поведение кнопки.
  beforeEach: () => {
    spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  },
  args: { code: "cctl login\ncctl setup npm", copyLabel: "Копировать команду установки" },
} satisfies Meta<typeof CodeBlock>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Login: Story = {
  name: "cctl login",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Копировать команду установки" }));
    await expect(navigator.clipboard.writeText).toHaveBeenCalledWith("cctl login\ncctl setup npm");
    await waitFor(() => expect(canvas.getByRole("region", { name: "Код" })).toBeVisible());
  },
};

export const Npmrc: Story = {
  name: ".npmrc с переменной токена",
  args: {
    code: NPMRC,
    copyLabel: "Копировать .npmrc",
    label: "Настройка .npmrc",
  },
};

export const LongLine: Story = {
  name: "Длинная строка без переноса",
  args: {
    code: "pnpm add @acme/ui@1.4.2 --registry=https://crane.acme.example/npm/ --config.strict-ssl=true --config.fetch-retries=3 --config.fetch-timeout=60000",
    label: "Команда установки",
  },
  decorators: [
    (Story) => (
      <div className="w-[360px]">
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("region", { name: "Команда установки" }),
    ).toHaveAttribute("tabindex", "0");
  },
};

export const Wrapped: Story = {
  name: "С переносом строк",
  args: { ...LongLine.args, wrap: true },
  decorators: LongLine.decorators ?? [],
};

export const Dark: Story = { globals: { theme: "dark" } };

export const MonoWithoutLigatures: Story = {
  name: "Моно: JetBrains Mono, лигатуры выключены (>=, ==>, !=)",
  args: {
    code: 'npm i @acme/ui@">=1.4.0 <2.0.0"  # a != b ==> c',
    copyLabel: "Копировать команду",
  },
  play: async ({ canvasElement }) => {
    await document.fonts.load('400 14px "JetBrains Mono Variable"', ">=");
    const style = getComputedStyle(within(canvasElement).getByRole("region", { name: "Код" }));
    await expect(style.fontFamily).toContain("JetBrains Mono Variable");
    await expect(style.fontVariantLigatures).toBe("none");
    await expect(style.fontFeatureSettings).toContain('"calt" 0');
    await expect(style.fontFeatureSettings).toContain('"liga" 0');
  },
};

const LONG =
  "pnpm add @acme/ui@1.4.2 --registry=https://crane.acme.example/npm/ --config.strict-ssl=true --config.fetch-retries=3 --config.fetch-timeout=60000 --config.audit-level=high";

/** Текст не заходит под кнопку копирования: правый край области кода левее левого края кнопки. */
async function expectNoOverlap(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  const region = canvas.getByRole("region", { name: "Команда установки" });
  const button = canvas.getByRole("button", { name: "Копировать команду установки" });
  await expect(region.getBoundingClientRect().right).toBeLessThanOrEqual(
    button.getBoundingClientRect().left,
  );
  await expect(
    button.parentElement ? getComputedStyle(button.parentElement).backgroundColor : "",
  ).not.toBe("rgba(0, 0, 0, 0)");
}

const narrow = [
  (Story: () => React.JSX.Element) => (
    <div className="w-[360px]">
      <Story />
    </div>
  ),
];

export const NoOverlapLight: Story = {
  name: "Длинная строка не заходит под кнопку",
  args: { code: LONG, label: "Команда установки" },
  decorators: narrow,
  play: async ({ canvasElement }) => {
    await expectNoOverlap(canvasElement);
  },
};

export const NoOverlapDark: Story = {
  name: "Длинная строка не заходит под кнопку, тёмная тема",
  args: { code: LONG, label: "Команда установки" },
  decorators: narrow,
  globals: { theme: "dark" },
  play: async ({ canvasElement }) => {
    await expectNoOverlap(canvasElement);
  },
};
