import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { type ProbeCheck, ProbeReport } from "~/shared/ui/data-display";

const PASS: ProbeCheck = { id: "reachable", label: "Хранилище доступно", status: "pass" };
const WARN: ProbeCheck = {
  id: "free-space",
  label: "Свободное место",
  status: "warn",
  detail: "Осталось 3 ГиБ из 500: рекомендуется не меньше 20 ГиБ",
};
const FAIL: ProbeCheck = {
  id: "write",
  label: "Запись в каталог",
  status: "fail",
  detail: "open /var/lib/crane/.probe: permission denied",
};
const SKIPPED: ProbeCheck = { id: "oidc", label: "Единый вход", status: "skipped" };

const meta = {
  title: "Данные/ProbeReport",
  component: ProbeReport,
  args: { state: "idle" },
  decorators: [
    (Story) => (
      <div className="w-[32rem] max-w-full">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof ProbeReport>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Idle: Story = { name: "До проверки: место занято, строк нет" };

export const Running: Story = {
  name: "Идёт проверка",
  args: { state: "running" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("Проверяем связь…")).toHaveAttribute(
      "aria-busy",
      "true",
    );
  },
};

export const RunningReserved: Story = {
  name: "Идёт проверка: место под 4 строки",
  args: { state: "running", expected: 4 },
};

export const Pass: Story = {
  name: "Всё успешно",
  args: { state: { checks: [PASS, { ...PASS, id: "auth", label: "Учётные данные приняты" }] } },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByText(
        "Проверка завершена: 2 успешно, 0 с предупреждениями, 0 с ошибками.",
      ),
    ).toBeVisible();
  },
};

export const Warn: Story = {
  name: "Предупреждение",
  args: { state: { checks: [PASS, WARN] } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText(WARN.detail ?? "")).toBeVisible();
    await expect(within(canvasElement).getByText("предупреждение")).toBeVisible();
  },
};

export const Fail: Story = {
  name: "Ошибка",
  args: { state: { checks: [FAIL] } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("ошибка")).toBeVisible();
    await expect(within(canvasElement).getByText(FAIL.detail ?? "")).toBeVisible();
  },
};

export const Skipped: Story = {
  name: "Пропущено: нейтрально",
  args: { state: { checks: [SKIPPED] } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("пропущено")).toBeVisible();
  },
};

export const Mixed: Story = {
  name: "Смешанный отчёт",
  args: { state: { checks: [PASS, WARN, FAIL, SKIPPED] } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("listitem")).toHaveLength(4);
    await expect(
      canvas.getByText("Проверка завершена: 1 успешно, 1 с предупреждением, 1 с ошибкой."),
    ).toBeVisible();
  },
};

export const RequestError: Story = {
  name: "Проверка не выполнилась",
  args: { state: { error: "Не удалось выполнить проверку: сервер не ответил" } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("alert")).toHaveTextContent("сервер не ответил");
  },
};

export const English: Story = {
  name: "EN",
  globals: { locale: "en" },
  args: { state: { checks: [PASS, WARN, FAIL, SKIPPED] } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      canvas.getByText("Check finished: 1 passed, 1 with a warning, 1 failed."),
    ).toBeVisible();
    await expect(canvas.getByText("skipped")).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { state: { checks: [PASS, WARN, FAIL, SKIPPED] } },
};
