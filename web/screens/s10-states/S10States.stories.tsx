import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ComponentProps } from "react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { StepUpDialog } from "~/features/step-up";
import { i18n } from "~/shared/lib/i18n";
import { showToast } from "~/shared/lib/state";
import { EmptyState, ProblemState } from "~/shared/ui/status";
import type { Banner } from "~/widgets/app-shell";
import { CommandPaletteView } from "~/widgets/command-palette";
import { type NotificationItem, NotificationsCenter } from "~/widgets/notifications-center";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";
import { PackagesBackdrop } from "./PackagesBackdrop";

const BELL = /^Уведомления/;
const MORE_BANNERS = /Ещё 2/;
const BODY = () => within(document.body);
const REQUEST_ID = "req_01JBX7Q4M2K9T5V8N3PZ6H1WRD";
function home() {
  return {
    label: i18n._(msg`На главную`),
    run: { href: "/" },
  } as const;
}

/** Крошки строятся при рендере: текст зависит от активной локали, а `args` читаются при загрузке модуля. */
function Frame(props: ComponentProps<typeof ScreenFrame>) {
  const { t } = useLingui();
  const breadcrumbs = {
    trail: [{ label: t`Каталог`, href: "/search" }],
    current: t`Мои пакеты`,
  };
  return <ScreenFrame {...props} breadcrumbs={breadcrumbs} />;
}

const meta = {
  title: "Экраны/S10 Состояния и оверлеи",
  component: ScreenFrame,
  parameters: screenParameters,
  globals: desktop,
  args: { active: "packages", breadcrumbs: { trail: [], current: "" }, children: null },
} satisfies Meta<typeof ScreenFrame>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Тёмный и мобильный варианты строятся из светлого desktop без дублирования. */
const dark = (story: Story): Story => ({
  ...story,
  globals: { ...desktop, theme: "dark" },
});
const phone = (story: Story): Story => ({ ...story, globals: { ...mobile } });

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

// ── 6. Палитра ──────────────────────────────────────────────────────────────

function paletteNav() {
  return [
    {
      id: "acme-pkgs",
      label: i18n._(msg`Пакеты workspace acme-platform`),
      href: "/packages?ws=acme-platform",
      hint: i18n._(msg`Раздел`),
    },
    {
      id: "acme-tokens",
      label: i18n._(msg`Токены acme-platform`),
      href: "/me/tokens?ws=acme-platform",
      hint: i18n._(msg`Доступ`),
    },
    {
      id: "audit",
      label: i18n._(msg`Журнал`),
      href: "/audit",
    },
  ] as const;
}

const PALETTE_PACKAGES = [
  { id: "acme-ui", name: "@acme/ui", format: "npm", latest: "1.4.2", href: "/packages/acme-ui" },
  {
    id: "acme-ui-tokens",
    name: "@acme/ui-tokens",
    format: "npm",
    latest: "0.9.14",
    href: "/packages/acme-ui-tokens",
  },
  {
    id: "acme-billing",
    name: "acme-billing-client",
    format: "PyPI",
    latest: "3.1.0",
    href: "/packages/acme-billing-client",
  },
  {
    id: "acme-payments",
    name: "org.acme:payments-core",
    format: "Maven",
    latest: "7.2.1",
    href: "/packages/payments-core",
  },
] as const;

function paletteActions() {
  return [
    {
      label: i18n._(msg`Выпустить токен для acme-ci`),
      run: { onSelect: fn() },
    },
    {
      label: i18n._(msg`Открыть диагностику по request_id`),
      run: { href: "/diagnostics" },
    },
    {
      label: i18n._(msg`Отозвать токены acme-platform`),
      run: { onSelect: fn() },
      stepUp: true,
      destructive: true,
    },
  ] as const;
}

const palette: Story = {
  name: "6 Командная палитра",
  render: (args) => (
    <>
      <Frame {...args}>
        <PackagesBackdrop />
      </Frame>
      <CommandPaletteView
        open
        onOpenChange={fn()}
        onQueryChange={fn()}
        onNavigate={fn()}
        navigation={paletteNav()}
        packages={PALETTE_PACKAGES}
        actions={paletteActions()}
      />
    </>
  ),
  play: async () => {
    const input = await BODY().findByRole("combobox");
    await userEvent.type(input, "acme");
    await userEvent.keyboard("{ArrowDown}");
    await waitFor(() =>
      expect(document.querySelector("[role=option][data-highlighted]")).not.toBeNull(),
    );
  },
};
export const Palette = palette;
export const PaletteDark = dark(palette);
export const PaletteMobile = phone(palette);

// ── 7. Центр уведомлений ────────────────────────────────────────────────────

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

function notificationItems(): readonly NotificationItem[] {
  return [
    {
      id: "n1",
      category: "security",
      tone: "error",
      title: i18n._(msg`P0 без действия: CVE-2026-31337 в @acme/ui 1.4.2`),
      time: ago(12),
      caption: i18n._(msg`@acme/ui · npm · SLA истёк`),
      href: "/findings/CVE-2026-31337",
      unread: true,
    },
    {
      id: "n2",
      category: "access",
      tone: "warning",
      title: i18n._(msg`Токен crp_9f8e7d6c истекает через 3 дня`),
      time: ago(95),
      caption: "ci-deploy · acme-platform",
      href: "/me/tokens/crp_9f8e7d6c",
      unread: true,
    },
    {
      id: "n3",
      category: "platform",
      tone: "warning",
      title: i18n._(msg`Квота billing: 85% хранилища`),
      time: ago(180),
      caption: i18n._(msg`workspace billing · 34 из 40 ГБ`),
      href: "/platform/storages",
      unread: true,
    },
    {
      id: "n4",
      category: "platform",
      tone: "warning",
      title: i18n._(msg`Upstream npmjs недоступен`),
      time: ago(240),
      caption: i18n._(msg`Данные от 25 окт, stale-if-error`),
      href: "/platform",
      unread: false,
    },
    {
      id: "n5",
      category: "security",
      tone: "success",
      title: i18n._(msg`lodash 4.17.21 выпущена из карантина`),
      time: ago(60 * 26),
      caption: "lodash · npm",
      href: "/packages/lodash",
      unread: false,
    },
    {
      id: "n6",
      category: "access",
      tone: "info",
      title: i18n._(msg`Петрова А. одобрила запрос доступа к acme-billing-client`),
      time: ago(60 * 30),
      href: "/me/actions",
      unread: false,
    },
  ];
}

const BELL_POSITION = "fixed top-3 right-14 z-30 lg:right-[68px]";

const notifications: Story = {
  name: "7 Центр уведомлений",
  render: (args) => (
    <>
      <Frame {...args}>
        <PackagesBackdrop />
      </Frame>
      {/* Центр сам владеет колокольчиком-триггером: ставим его поверх колокольчика рамки. */}
      <div className={BELL_POSITION}>
        <NotificationsCenter
          items={notificationItems()}
          unread={3}
          unreadByCategory={{ security: 1, access: 1, platform: 1 }}
          onCategoryChange={fn()}
          onMarkRead={fn()}
          onReadAll={fn()}
          loadMore={{ label: i18n._(msg`Загрузить ещё`), run: { onSelect: fn() } }}
        />
      </div>
    </>
  ),
  play: async () => {
    const bells = await BODY().findAllByRole("button", { name: BELL });
    const bell = bells.at(-1);
    if (bell === undefined) {
      throw new Error("Колокольчик центра не найден");
    }
    await userEvent.click(bell);
    const popup = await BODY().findByRole("dialog", { name: "Уведомления" });
    await waitFor(() => expect(popup).toContainElement(document.activeElement as HTMLElement));
  },
};
export const Notifications = notifications;
export const NotificationsDark = dark(notifications);
export const NotificationsMobile = phone(notifications);

// ── 8. Step-up ──────────────────────────────────────────────────────────────

const stepUp: Story = {
  name: "8 Step-up: подтвердите вход",
  render: (args) => (
    <>
      <Frame {...args}>
        <PackagesBackdrop />
      </Frame>
      <StepUpDialog
        open
        onOpenChange={fn()}
        actionLabel={i18n._(msg`Раскрыть данные события`)}
        onContinue={fn()}
      />
    </>
  ),
  play: async () => {
    await expect(
      await BODY().findByRole("button", { name: "Продолжить в Keycloak" }),
    ).toBeVisible();
  },
};
export const StepUp = stepUp;
export const StepUpDark = dark(stepUp);
export const StepUpMobile = phone(stepUp);

// ── 9. Стек баннеров ────────────────────────────────────────────────────────

function bannerItems(): readonly Banner[] {
  return [
    {
      id: "p0",
      tone: "error",
      urgent: true,
      text: i18n._(msg`P0 без действия более 1 часа: CVE-2026-31337 в @acme/ui 1.4.2`),
      action: {
        label: i18n._(msg`Открыть находку`),
        run: { href: "/findings/CVE-2026-31337" },
      },
    },
    {
      id: "read-only",
      tone: "warning",
      text: i18n._(
        msg`Только чтение: основная БД недоступна, публикация отключена. Повторите через 2 мин`,
      ),
      action: {
        label: i18n._(msg`Статус платформы`),
        run: { href: "/platform" },
      },
    },
    {
      id: "sod",
      tone: "warning",
      text: i18n._(
        msg`Включён режим разделения обязанностей: действия требуют второго подтверждения`,
      ),
    },
    {
      id: "upstream",
      tone: "warning",
      text: i18n._(
        msg`Upstream npmjs недоступен: показаны данные от 25 окт (stale-if-error, до 7 дней)`,
      ),
    },
  ];
}

const banners: Story = {
  name: "9 Стек баннеров (2 + «Ещё 2»)",
  render: (args) => (
    <Frame {...args} banners={bannerItems()}>
      <PackagesBackdrop />
    </Frame>
  ),
  play: async () => {
    await expect(await BODY().findByText(MORE_BANNERS)).toBeVisible();
  },
};
export const Banners = banners;
export const BannersDark = dark(banners);
export const BannersMobile = phone(banners);

// ── 10. Тосты ───────────────────────────────────────────────────────────────

const toasts: Story = {
  name: "10 Тосты: успех и ошибка с request_id",
  render: (args) => (
    <Frame {...args}>
      <PackagesBackdrop />
    </Frame>
  ),
  play: async () => {
    showToast.success("Токен отозван. Применится в течение 5 секунд");
    showToast.error({ title: "Не удалось выпустить версию 1.4.2", requestId: REQUEST_ID });
    // Ошибочные тосты не исчезают сами и переживают переключение story: видимым может быть любой из них,
    // поэтому проверяем наличие, а не видимость.
    await waitFor(async () =>
      expect(await BODY().findAllByText("Не удалось выпустить версию 1.4.2")).not.toHaveLength(0),
    );
    await expect(
      await BODY().findAllByRole("button", { name: "Копировать request_id" }),
    ).not.toHaveLength(0);
  },
};
export const Toasts = toasts;
export const ToastsDark = dark(toasts);
export const ToastsMobile = phone(toasts);

export const EmptyEnglish: Story = {
  ...empty,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async () => {
    await expect(await BODY().findByText("No packages yet")).toBeInTheDocument();
    await expect(await BODY().findByRole("link", { name: "How to publish" })).toBeVisible();
  },
};
