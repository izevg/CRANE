import type { Meta, StoryObj } from "@storybook/react-vite";
import { useQueryClient } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { useState } from "react";
import { configure, expect, getConfig, userEvent, waitFor, within } from "storybook/test";
import { type ApiOverrides, ApiProblem, type Status } from "~/shared/api";
import { createAppRouter } from "./router";

// Чанк страницы грузится лениво: на холодном кэше Vite это дольше секунды, которую ждёт findBy по умолчанию.
const LOAD_TIMEOUT_MS = 10_000;

const VIEWPORTS = {
  desktop: { name: "Десктоп 1440", styles: { width: "1440px", height: "900px" } },
  mobile: { name: "Телефон 390", styles: { width: "390px", height: "844px" } },
};

/** Приложение на памяти по адресу `url`; клиент запросов даёт декоратор витрины (свежий на историю, retry: false). */
function AppAt({ url }: { url: string }) {
  const queryClient = useQueryClient();
  const [router] = useState(() =>
    createAppRouter({ queryClient, history: createMemoryHistory({ initialEntries: [url] }) }),
  );
  return <RouterProvider router={router} />;
}

const meta = {
  title: "Приложение/Маршруты",
  component: AppAt,
  parameters: { layout: "fullscreen", viewport: { options: VIEWPORTS } },
  globals: { viewport: { value: "desktop", isRotated: false } },
  args: { url: "/status" },
  beforeEach: () => {
    const previous = getConfig().asyncUtilTimeout;
    configure({ asyncUtilTimeout: LOAD_TIMEOUT_MS });
    return () => {
      configure({ asyncUtilTimeout: previous });
    };
  },
} satisfies Meta<typeof AppAt>;

export default meta;
type Story = StoryObj<typeof meta>;

const never = (): Promise<never> => new Promise(() => undefined);

export const StatusSuccess: Story = {
  name: "E15 Статус системы: данные, плашка демо",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Статус системы" }),
    ).toBeVisible();
    await expect(await canvas.findByText("Scanner worker")).toBeVisible();
    await expect(canvas.getAllByRole("note")).toHaveLength(1);
    await expect(
      canvas.getByRole("link", { name: "Статус системы", current: "page" }),
    ).toBeVisible();
  },
};

export const StatusLoading: Story = {
  name: "E15: загрузка — скелетон только в main, шелл на месте",
  parameters: { api: { getStatus: never } satisfies ApiOverrides },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("Загрузка раздела")).toBeInTheDocument();
    await expect(canvas.getByRole("navigation", { name: "Основное меню" })).toBeVisible();
    await expect(canvas.queryByRole("heading", { level: 1 })).toBeNull();
  },
};

const REFRESH_FAILED = "Не удалось обновить данные. Показаны последние полученные";
const SMALL_STATUS: Status = {
  version: "0.3.0",
  mode: "normal",
  feeds_max_lag_s: 30,
  siem_export_lag_s: null,
  components: [{ name: "PostgreSQL", kind: "database", state: "ok", latency_ms: 4 }],
  _links: { self: { href: "/status" } },
};
let statusCalls = 0;

export const StatusRefreshFailed: Story = {
  name: "E15: обновление не удалось — данные остаются, «Повторить» перечитывает",
  // loader читает данные (1-й вызов), страница при монтировании перечитывает их (2-й) — он и падает
  beforeEach: () => {
    statusCalls = 0;
  },
  parameters: {
    api: {
      getStatus: () => {
        statusCalls += 1;
        if (statusCalls === 2) {
          throw new ApiProblem({ kind: "problem", status: 500, title: "Сбой" });
        }
        return SMALL_STATUS;
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(REFRESH_FAILED)).toBeVisible();
    await expect(canvas.getByText("PostgreSQL")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Повторить" }));
    await waitFor(() => expect(canvas.queryByText(REFRESH_FAILED)).toBeNull());
  },
};

export const StatusError: Story = {
  name: "E15: ошибка маршрута — текст сервера, request_id, «Повторить»",
  parameters: {
    api: {
      getStatus: () => {
        throw new ApiProblem({
          kind: "problem",
          status: 500,
          title: "Хранилище недоступно",
          detail: "Не удаётся прочитать состояние компонентов",
          remediation: ["Подождите минуту и повторите"],
          requestId: "req_01JBX7Q4M2K9T5V8N3PZ6H1WRD",
        });
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Хранилище недоступно" }),
    ).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Повторить" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Основное меню" })).toBeVisible();
  },
};

export const StatusPlanned: Story = {
  name: "E15: production без демо — «Раздел появится в v0.4.0»",
  parameters: {
    api: {
      getStatus: () => {
        throw new ApiProblem({ kind: "planned", status: 0, replacedIn: "v0.4.0" });
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    await expect(
      await within(canvasElement).findByRole("heading", {
        level: 1,
        name: "Раздел появится в v0.4.0",
      }),
    ).toBeVisible();
  },
};

export const ServerDown: Story = {
  name: "Сервер CRANE недоступен: в области содержимого",
  parameters: {
    api: {
      getStatus: () => {
        throw new ApiProblem({ kind: "network", status: 0 });
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Сервер CRANE недоступен" }),
    ).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Основное меню" })).toBeVisible();
  },
};

export const ShellFailure: Story = {
  name: "Профиль не получен: полностраничный сбой без шелла",
  args: { url: "/status" },
  parameters: {
    api: {
      getMe: () => {
        throw new ApiProblem({ kind: "network", status: 0 });
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Сервер CRANE недоступен" }),
    ).toBeVisible();
    await expect(canvas.queryByRole("navigation", { name: "Основное меню" })).toBeNull();
    await expect(canvas.getAllByRole("main")).toHaveLength(1);
  },
};

export const NotFound: Story = {
  name: "404: адрес без маршрута, внутри шелла",
  args: { url: "/projects/ghost" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Не найдено или нет доступа" }),
    ).toBeVisible();
    await expect(canvas.getByRole("link", { name: "На главную" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Основное меню" })).toBeVisible();
  },
};

export const LoginFirst: Story = {
  name: "A1 /login: первый вход — без шелла, без кнопки",
  args: { url: "/login" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Войдите, чтобы продолжить" }),
    ).toBeVisible();
    await expect(canvas.getByText("Вход появится в следующей версии CRANE")).toBeVisible();
    await expect(canvas.queryByRole("button")).toBeNull();
    await expect(canvas.queryByRole("navigation")).toBeNull();
  },
};

export const LoginSessionEnded: Story = {
  name: "A1 /login: сессия отозвана",
  args: { url: "/login?reason=session_revoked&redirect=%2Fstatus" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Сессия завершена" }),
    ).toBeVisible();
    await expect(
      canvas.getByText("Сессия отозвана. Вход появится в следующей версии CRANE"),
    ).toBeVisible();
  },
};

export const Mobile: Story = {
  name: "E15 на телефоне 390",
  globals: { viewport: { value: "mobile", isRotated: false } },
  play: async ({ canvasElement }) => {
    await expect(
      await within(canvasElement).findByRole("heading", { level: 1, name: "Статус системы" }),
    ).toBeVisible();
  },
};

export const StatusEnglish: Story = {
  name: "EN: E15, lang и заголовок документа",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "System status" }),
    ).toBeVisible();
    await expect(document.documentElement.lang).toBe("en");
    await expect(document.title).toBe("System status — CRANE");
    await expect(canvas.getByText("Demo data")).toBeVisible();
  },
};

export const NotFoundEnglish: Story = {
  name: "EN: 404",
  args: { url: "/nothing-here" },
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    await expect(
      await within(canvasElement).findByRole("heading", {
        level: 1,
        name: "Not found or no access",
      }),
    ).toBeVisible();
  },
};
