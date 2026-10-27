import { msg } from "@lingui/core/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor } from "storybook/test";
import { StepUpDialog } from "~/features/step-up";
import { i18n } from "~/shared/lib/i18n";
import { CommandPaletteView } from "~/widgets/command-palette";
import { type NotificationItem, NotificationsCenter } from "~/widgets/notifications-center";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, screenParameters } from "../frame/screen";
import { PackagesBackdrop } from "./PackagesBackdrop";
import { Frame } from "./S10Frame";
import { BELL, BODY, dark, phone } from "./S10Shared";

const meta = {
  title: "Экраны/S10 Состояния и оверлеи",
  component: ScreenFrame,
  parameters: screenParameters,
  globals: desktop,
  args: { active: "packages", breadcrumbs: { trail: [], current: "" }, children: null },
} satisfies Meta<typeof ScreenFrame>;

export default meta;
type Story = StoryObj<typeof meta>;

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
