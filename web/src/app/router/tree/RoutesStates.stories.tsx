import type { Meta, StoryObj } from "@storybook/react-vite";
import { configure, expect, getConfig, spyOn, within } from "storybook/test";
import { type ApiOverrides, ApiProblem, type Status } from "~/shared/api";
import { ChunkLoadError } from "~/shared/lib/route";
import { AppAt } from "../harness/AppAt";
import { refetchStale } from "../harness/refresh";
import { markReload } from "../recovery/chunk";

const LOAD_TIMEOUT_MS = 10_000;
const INTERNAL_DETAIL = /внутренняя деталь/;

const meta = {
  title: "Приложение/Состояния маршрутов",
  component: AppAt,
  parameters: { layout: "fullscreen" },
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

const SMALL_STATUS: Status = {
  version: "0.3.0",
  mode: "normal",
  feeds_max_lag_s: 30,
  siem_export_lag_s: null,
  components: [{ name: "PostgreSQL", kind: "database", state: "ok", latency_ms: 4 }],
  _links: { self: { href: "/status" } },
};

export const ChunkFailure: Story = {
  name: "Сбой чанка (повторный): «Не удалось загрузить раздел» в области содержимого",
  // повторный сбой в окне после перезагрузки: метка стоит, поэтому страница не перезагружается сама
  beforeEach: () => {
    markReload();
    return () => {
      sessionStorage.clear();
    };
  },
  parameters: {
    api: {
      getStatus: () => {
        throw new ChunkLoadError(new TypeError("Failed to fetch dynamically imported module"));
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Не удалось загрузить раздел" }),
    ).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Обновить страницу" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Основное меню" })).toBeVisible();
  },
};

export const RenderFailure: Story = {
  name: "Сбой без problem+json: «Не удалось открыть страницу», без текста исключения",
  parameters: {
    api: {
      getStatus: () => {
        throw new TypeError("внутренняя деталь, которой не должно быть на экране");
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Не удалось открыть страницу" }),
    ).toBeVisible();
    await expect(canvas.queryByText(INTERNAL_DETAIL)).toBeNull();
  },
};

export const OfflineNetwork: Story = {
  name: "Нет сети у пользователя: «Нет соединения», повтор сам",
  beforeEach: () => {
    spyOn(navigator, "onLine", "get").mockReturnValue(false);
  },
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
      await canvas.findByRole("heading", { level: 1, name: "Нет соединения" }),
    ).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Повторить" })).toBeVisible();
  },
};

let statusCalls = 0;

export const BackgroundSessionEnded: Story = {
  name: "401 в фоне: баннер «Сессия завершена», страница и данные на месте",
  // loader читает данные (1-й вызов), перечитывание устаревших данных (2-й) возвращает 401
  beforeEach: () => {
    statusCalls = 0;
  },
  parameters: {
    api: {
      getStatus: () => {
        statusCalls += 1;
        if (statusCalls === 2) {
          throw new ApiProblem({ kind: "unauthenticated", status: 401 });
        }
        return SMALL_STATUS;
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("heading", { level: 1, name: "Статус системы" });
    await refetchStale();
    await expect(
      await canvas.findByText("Сессия завершена. Вход в веб-консоль появится в CRANE 0.4.0."),
    ).toBeVisible();
    await expect(canvas.getByRole("heading", { level: 1, name: "Статус системы" })).toBeVisible();
    await expect(canvas.getByText("PostgreSQL")).toBeVisible();
    await expect(canvas.queryByRole("link", { name: "Войти снова" })).toBeNull();
  },
};

export const ShellPlanned: Story = {
  name: "Production без демо: шелл — «Веб-консоль появится в CRANE 0.4.0»",
  parameters: {
    api: {
      getMe: () => {
        throw new ApiProblem({ kind: "planned", status: 0, replacedIn: "v0.4.0" });
      },
    } satisfies ApiOverrides,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      await canvas.findByRole("heading", { level: 1, name: "Веб-консоль появится в CRANE 0.4.0" }),
    ).toBeVisible();
    await expect(canvas.queryByRole("navigation", { name: "Основное меню" })).toBeNull();
  },
};
