import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { ProblemState } from "~/shared/ui/status";

const REQUEST_ID = "req_01JBX7Q4M2K9T5V8N3PZ6H1WRD";
const RULE = /Правило/;
const CONTACT = /Контакт/;
const REQUEST_ACCESS = /Запросить доступ/;
const onRetry = fn();
const onRequestAccess = fn();

const meta = {
  title: "Статус/ProblemState",
  component: ProblemState,
} satisfies Meta<typeof ProblemState>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NotFound: Story = {
  name: "404: нейтральный экран",
  args: {
    problem: {
      kind: "not-found",
      requestId: REQUEST_ID,
      search: (
        <input
          type="search"
          aria-label="Поиск пакета"
          placeholder="Имя пакета"
          className="h-9 w-full rounded-md border border-input bg-card px-3"
        />
      ),
      home: { label: "На главную", run: { href: "/" } },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { name: "Не найдено или нет доступа" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "На главную" })).toBeVisible();
    await expect(canvas.queryByText(RULE)).toBeNull();
    await expect(canvas.queryByText(CONTACT)).toBeNull();
    await expect(canvas.queryByRole("button", { name: REQUEST_ACCESS })).toBeNull();
  },
};

export const Forbidden: Story = {
  name: "403: правило, request_id, контакт, шаги",
  args: {
    problem: {
      kind: "forbidden",
      ruleId: "repo.publish.require-maintainer",
      requestId: REQUEST_ID,
      contact: "Петрова А. (владелец workspace billing)",
      remediation: [
        "Запросите роль maintainer у владельца workspace",
        "Повторите публикацию после подтверждения",
      ],
      requestAccess: { label: "Запросить доступ", run: { onSelect: onRequestAccess } },
      home: { label: "На главную", run: { href: "/" } },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Правило: repo.publish.require-maintainer")).toBeVisible();
    await expect(canvas.getAllByRole("listitem")).toHaveLength(2);
    await userEvent.click(canvas.getByRole("button", { name: "Запросить доступ" }));
    await expect(onRequestAccess).toHaveBeenCalledTimes(1);
  },
};

export const ForbiddenWithoutRequestAccess: Story = {
  name: "403: без «Запросить доступ»",
  args: {
    problem: {
      kind: "forbidden",
      ruleId: "ws-billing-readers",
      requestId: REQUEST_ID,
      remediation: [],
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("button", { name: "Запросить доступ" })).toBeNull();
    await expect(canvas.queryByRole("list")).toBeNull();
  },
};

export const ErrorWithRetry: Story = {
  name: "Ошибка: шаги, «Диагностика» и «Повторить»",
  args: {
    problem: {
      kind: "error",
      title: "Размер публикации превышает лимит",
      detail: "14 МБ при лимите 10 МБ",
      ruleId: "max-publish-size",
      requestId: REQUEST_ID,
      remediation: ["Уменьшите архив", "Попросите владельца workspace поднять лимит"],
      diagnostics: {
        label: "Диагностика",
        run: { href: "/diagnostics/req_01JBX7Q4M2K9T5V8N3PZ6H1WRD" },
      },
      retry: { label: "Повторить", run: { onSelect: onRetry } },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Повторить" }));
    await expect(onRetry).toHaveBeenCalledTimes(1);
    await expect(canvas.getByRole("link", { name: "Диагностика" })).toBeVisible();
  },
};

export const ErrorWithoutRetry: Story = {
  name: "Ошибка: без «Повторить»",
  args: {
    problem: {
      kind: "error",
      title: "Не удалось загрузить версии пакета",
      requestId: REQUEST_ID,
      remediation: [],
    },
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("button", { name: "Повторить" })).toBeNull();
  },
};

export const Offline: Story = {
  name: "Нет соединения",
  args: { problem: { kind: "offline", retry: { label: "Повторить", run: { onSelect: onRetry } } } },
};

export const Session: Story = {
  name: "Сессия завершена",
  args: { problem: { kind: "session", signIn: { label: "Войти снова", run: { href: "/login" } } } },
};

export const Compact: Story = {
  name: "Компактный, заголовок третьего уровня",
  args: {
    compact: true,
    headingLevel: 3,
    problem: { kind: "offline" },
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("heading", { level: 3 })).toBeVisible();
  },
};

export const PageHeading: Story = {
  name: "Страница: заголовок первого уровня",
  args: {
    headingLevel: 1,
    problem: { kind: "session", signIn: { label: "Войти снова", run: { href: "/login" } } },
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("heading", { level: 1 })).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: Forbidden.args,
};
