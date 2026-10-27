import { msg } from "@lingui/core/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import { EmptyState, ProblemState } from "~/shared/ui/status";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, screenParameters } from "../frame/screen";
import { PackagesBackdrop } from "./PackagesBackdrop";
import { Frame } from "./S10Frame";
import { BODY, dark, home, phone, REQUEST_ID } from "./S10Shared";

const meta = {
  title: "Экраны/S10 Состояния и оверлеи",
  component: ScreenFrame,
  parameters: screenParameters,
  globals: desktop,
  args: { active: "packages", breadcrumbs: { trail: [], current: "" }, children: null },
} satisfies Meta<typeof ScreenFrame>;

export default meta;
type Story = StoryObj<typeof meta>;

// ── 1. Загрузка ─────────────────────────────────────────────────────────────

const loading: Story = {
  name: "1 Загрузка (8 скелетон-строк)",
  render: (args) => (
    <Frame {...args}>
      <PackagesBackdrop loading />
    </Frame>
  ),
  play: async () => {
    await expect(await BODY().findByText("Загрузка")).toBeInTheDocument();
  },
};
export const Loading = loading;
export const LoadingDark = dark(loading);
export const LoadingMobile = phone(loading);

// ── 2. Пусто ────────────────────────────────────────────────────────────────

const empty: Story = {
  name: "2 Пусто (первый запуск)",
  render: (args) => (
    <Frame {...args}>
      <PackagesBackdrop>
        <EmptyState
          variant="first-run"
          title={i18n._(msg`Пакетов пока нет`)}
          description={i18n._(
            msg`Опубликуйте первый пакет в репозиторий вашего workspace или перенесите существующие из Nexus.`,
          )}
          primary={{ label: i18n._(msg`Как опубликовать`), run: { href: "/help/publish" } }}
          secondary={{ label: i18n._(msg`Импорт из Nexus`), run: { href: "/import/nexus" } }}
        />
      </PackagesBackdrop>
    </Frame>
  ),
  play: async () => {
    await expect(await BODY().findByRole("link", { name: "Как опубликовать" })).toBeVisible();
  },
};
export const Empty = empty;
export const EmptyDark = dark(empty);
export const EmptyMobile = phone(empty);

// ── 3. Ошибка problem+json ──────────────────────────────────────────────────

const error: Story = {
  name: "3 Ошибка problem+json",
  render: (args) => (
    <Frame {...args}>
      <div className="p-4 lg:p-6">
        <ProblemState
          headingLevel={1}
          problem={{
            kind: "error",
            title: i18n._(msg`Не удалось опубликовать`),
            detail: i18n._(msg`Превышен размер публикации: 14 МБ при лимите 10 МБ`),
            ruleId: "max-publish-size",
            requestId: REQUEST_ID,
            remediation: [
              i18n._(
                msg`Уменьшите пакет: уберите артефакты сборки и тестовые данные из файлов публикации`,
              ),
              i18n._(msg`Попросите владельца репозитория npm-hosted поднять лимит размера`),
            ],
            diagnostics: {
              label: i18n._(msg`Открыть диагностику`),
              run: { href: `/diagnostics/${REQUEST_ID}` },
            },
            retry: { label: i18n._(msg`Повторить`), run: { onSelect: fn() } },
          }}
        />
      </div>
    </Frame>
  ),
  play: async () => {
    await expect(
      await BODY().findByRole("heading", { name: "Не удалось опубликовать" }),
    ).toBeVisible();
  },
};
export const ProblemError = error;
export const ProblemErrorDark = dark(error);
export const ProblemErrorMobile = phone(error);

// ── 4. 403 ──────────────────────────────────────────────────────────────────

const forbidden: Story = {
  name: "4 Недостаточно прав (403)",
  render: (args) => (
    <Frame {...args}>
      <div className="p-4 lg:p-6">
        <ProblemState
          headingLevel={1}
          problem={{
            kind: "forbidden",
            ruleId: "ws-billing-readers",
            requestId: REQUEST_ID,
            contact: i18n._(msg`Петрова А. (владелец workspace billing)`),
            remediation: [
              i18n._(msg`Запросите роль maintainer у владельца workspace`),
              i18n._(msg`Повторите действие после подтверждения доступа`),
            ],
            requestAccess: { label: i18n._(msg`Запросить доступ`), run: { onSelect: fn() } },
            home: home(),
          }}
        />
      </div>
    </Frame>
  ),
  play: async () => {
    await expect(await BODY().findByRole("heading", { name: "Недостаточно прав" })).toBeVisible();
  },
};
export const Forbidden = forbidden;
export const ForbiddenDark = dark(forbidden);
export const ForbiddenMobile = phone(forbidden);

// ── 5. 404 ──────────────────────────────────────────────────────────────────

const notFound: Story = {
  name: "5 Не найдено или нет доступа (404)",
  render: (args) => (
    <Frame {...args}>
      <div className="p-4 lg:p-6">
        <ProblemState
          headingLevel={1}
          problem={{
            kind: "not-found",
            requestId: REQUEST_ID,
            search: (
              <input
                type="search"
                aria-label={i18n._(msg`Поиск пакета`)}
                placeholder={i18n._(msg`Имя пакета`)}
                className="h-9 w-full rounded-md border border-input bg-card px-3 text-body"
              />
            ),
            home: home(),
          }}
        />
      </div>
    </Frame>
  ),
  play: async () => {
    await expect(
      await BODY().findByRole("heading", { name: "Не найдено или нет доступа" }),
    ).toBeVisible();
  },
};
export const NotFound = notFound;
export const NotFoundDark = dark(notFound);
export const NotFoundMobile = phone(notFound);

export const EmptyEnglish: Story = {
  ...empty,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async () => {
    await expect(await BODY().findByText("No packages yet")).toBeInTheDocument();
    await expect(await BODY().findByRole("link", { name: "How to publish" })).toBeVisible();
  },
};
