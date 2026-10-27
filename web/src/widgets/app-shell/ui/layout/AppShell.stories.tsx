import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  ActivityIcon,
  BellIcon,
  BuildingIcon,
  HistoryIcon,
  KeyRoundIcon,
  LayoutDashboardIcon,
  PackageIcon,
  ScaleIcon,
  ScrollTextIcon,
  SearchIcon,
  SettingsIcon,
  ShieldAlertIcon,
  StarIcon,
} from "lucide-react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { Button } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { AppShell, type Banner } from "~/widgets/app-shell";

const COLLAPSE = /Свернуть:/;
const AVATAR = /аккаунт/;

const VIEWPORTS = {
  desktop: { name: "Десктоп 1440", styles: { width: "1440px", height: "900px" } },
  mobile: { name: "Телефон 390", styles: { width: "390px", height: "844px" } },
  mobile320: { name: "Телефон 320", styles: { width: "320px", height: "640px" } },
};

const READ_ONLY: Banner = {
  id: "read-only",
  tone: "warning",
  text: "Только чтение: основная БД недоступна, публикация временно отключена. Повторите через 2 мин",
};
const SOD: Banner = {
  id: "sod",
  tone: "warning",
  text: "Включён режим разделения обязанностей: действия требуют второго подтверждения",
};
const UPSTREAM: Banner = {
  id: "upstream",
  tone: "warning",
  text: "Upstream npmjs недоступен: показаны данные от 25 окт (stale-if-error, до 7 дней)",
};
const LLM: Banner = {
  id: "llm",
  tone: "info",
  text: "Ассистенты недоступны, поиск работает без семантики",
};
const P0: Banner = {
  id: "p0",
  tone: "error",
  urgent: true,
  text: "P0 без действия более 1 часа: CVE-2026-31337 в @acme/ui 1.4.2",
  action: { label: "Открыть находку", run: { href: "/findings/CVE-2026-31337" } },
};

const meta = {
  title: "Шелл/AppShell",
  component: AppShell,
  parameters: { layout: "fullscreen", viewport: { options: VIEWPORTS } },
  globals: { viewport: { value: "desktop", isRotated: false } },
  args: {
    sidebar: {
      // Группы строятся снаружи из `_links` ответа GET /me, а не из названий ролей.
      groups: [
        {
          id: "registry",
          label: "Реестр",
          items: [
            { id: "search", label: "Поиск", href: "/search", icon: SearchIcon, active: true },
            { id: "favorites", label: "Избранное", href: "/favorites", icon: StarIcon },
            { id: "recent", label: "Недавние", href: "/recent", icon: HistoryIcon },
            { id: "packages", label: "Пакеты", href: "/packages", icon: PackageIcon },
          ],
        },
        {
          id: "security",
          label: "Безопасность",
          items: [
            { id: "findings", label: "Находки", href: "/findings", icon: ShieldAlertIcon },
            { id: "licenses", label: "Лицензии", href: "/licenses", icon: ScaleIcon },
            { id: "audit", label: "Журнал", href: "/audit", icon: ScrollTextIcon },
          ],
        },
        {
          id: "platform",
          label: "Платформа",
          items: [
            { id: "overview", label: "Обзор", href: "/platform", icon: LayoutDashboardIcon },
            { id: "health", label: "Состояние", href: "/platform/health", icon: ActivityIcon },
          ],
        },
      ],
      footerItems: [
        { id: "tokens", label: "Мои токены", href: "/me/tokens", icon: KeyRoundIcon },
        { id: "actions", label: "Мои действия", href: "/me/actions", icon: HistoryIcon },
        { id: "settings", label: "Настройки", href: "/settings", icon: SettingsIcon },
      ],
      helpHref: "/help",
      account: { name: "Анна Смирнова", role: "владелец библиотеки" },
    },
    topbar: {
      breadcrumbs: { trail: [{ label: "Поиск", href: "/search" }], current: "Результаты" },
      onOpenPalette: fn(),
      workspace: (
        <Button variant="secondary" size="sm">
          <BuildingIcon aria-hidden="true" className="sm:hidden" />
          <span className="max-sm:sr-only">Workspace: platform</span>
        </Button>
      ),
      status: <StatusBadge tone="success" label="Режим: норма" />,
      notifications: (
        <Button variant="ghost" size="icon" aria-label="Уведомления: 3">
          <BellIcon aria-hidden="true" />
        </Button>
      ),
      account: (
        <Button variant="ghost" size="icon" aria-label="АС, аккаунт Анны Смирновой">
          АС
        </Button>
      ),
    },
    onCollapseBanner: fn(),
    children: (
      <>
        <h1 className="text-h1">Поиск</h1>
        <p className="mt-1 text-body text-muted-foreground">Пакеты всех форматов, доступные вам</p>
      </>
    ),
  },
} satisfies Meta<typeof AppShell>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Full: Story = {
  name: "Полная композиция (S1)",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const nav = canvas.getByRole("navigation", { name: "Основное меню" });
    await expect(within(nav).getByRole("link", { name: "Поиск" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await expect(canvas.getByRole("link", { name: "Справка" })).toBeVisible();
    await expect(canvas.getByRole("main")).toContainElement(
      canvas.getByRole("heading", { level: 1, name: "Поиск" }),
    );
  },
};

export const SkipLink: Story = {
  name: "Skip-link переводит фокус на main",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.tab();
    await expect(canvas.getByRole("link", { name: "К содержимому" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    await expect(canvas.getByRole("main")).toHaveFocus();
  },
};

export const CollapseSidebar: Story = {
  name: "Свёртка сайдбара: aria-expanded",
  play: async ({ canvasElement }) => {
    const toggle = within(canvasElement).getByRole("button", { name: "Главное меню" });
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(toggle);
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
  },
};

export const OneBanner: Story = {
  name: "Баннеры: один",
  args: { banners: [READ_ONLY] },
};

export const TwoBanners: Story = {
  name: "Баннеры: два",
  args: { banners: [READ_ONLY, LLM] },
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: COLLAPSE }));
  },
};

export const FourBanners: Story = {
  name: "Баннеры: четыре, «Ещё 2» раскрывает",
  args: { banners: [READ_ONLY, SOD, UPSTREAM, LLM] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByText(UPSTREAM.text)).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Ещё 2" }));
    await expect(canvas.getByText(UPSTREAM.text)).toBeVisible();
  },
};

export const UrgentP0: Story = {
  name: "Баннеры: срочный P0",
  args: { banners: [P0, READ_ONLY] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("alert")).toHaveTextContent(P0.text);
  },
};

/** Страница не шире окна: горизонтального скролла нет, а аккаунт целиком в области просмотра (WCAG 1.4.10). */
async function expectNoHorizontalScroll(canvasElement: HTMLElement) {
  const root = canvasElement.ownerDocument.documentElement;
  await expect(root.scrollWidth).toBeLessThanOrEqual(window.innerWidth);
  const avatar = within(canvasElement).getByRole("button", { name: AVATAR });
  await expect(avatar.getBoundingClientRect().right).toBeLessThanOrEqual(window.innerWidth);
}

export const Mobile390Geometry: Story = {
  name: "Телефон 390: topbar помещается, скролла нет",
  globals: { viewport: { value: "mobile", isRotated: false } },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(window.innerWidth).toBe(390));
    await expectNoHorizontalScroll(canvasElement);
  },
};

const LONG_TRAIL = {
  trail: [
    { label: "Мои пакеты", href: "/packages" },
    { label: "@acme/design-system-components", href: "/packages/acme-design-system" },
    { label: "Версии и состояния публикации", href: "/packages/acme-design-system/versions" },
  ],
  current: "Версия 1.4.2-rc.1 и её происхождение",
};

/** Длинный путь крошек не раздвигает панель: на телефоне виден только текущий раздел с обрезкой. */
export const Mobile320LongTrail: Story = {
  name: "Телефон 320: длинный путь крошек, виден только текущий раздел",
  globals: { viewport: { value: "mobile320", isRotated: false } },
  args: { topbar: { ...meta.args.topbar, breadcrumbs: LONG_TRAIL } },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(window.innerWidth).toBe(320));
    await expectNoHorizontalScroll(canvasElement);
    const nav = within(canvasElement).getByRole("navigation", { name: "Навигационная цепочка" });
    await expect(within(nav).getByText(LONG_TRAIL.current)).toBeVisible();
    await expect(
      within(nav).getByRole("link", { name: "Мои пакеты", hidden: true }),
    ).not.toBeVisible();
  },
};

export const Mobile320Geometry: Story = {
  name: "Телефон 320: topbar помещается, скролла нет",
  globals: { viewport: { value: "mobile320", isRotated: false } },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(window.innerWidth).toBe(320));
    await expectNoHorizontalScroll(canvasElement);
  },
};

/** Остаётся открытой в момент axe-проверки. */
export const MobileSheet: Story = {
  name: "Телефон: сайдбар в Sheet",
  globals: { viewport: { value: "mobile", isRotated: false } },
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Главное меню" }));
    const dialog = await within(document.body).findByRole("dialog", { name: "Главное меню" });
    // Sheet въезжает с opacity: ждём конца входа
    await waitFor(() =>
      expect(within(dialog).getByRole("link", { name: "Справка" })).toBeVisible(),
    );
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark", viewport: { value: "desktop", isRotated: false } },
  args: { banners: [P0, READ_ONLY] },
  play: async () => {
    await waitFor(() => expect(document.documentElement).toHaveAttribute("data-theme", "dark"));
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("navigation", { name: "Primary menu" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "Help" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "Skip to content" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Theme: System" })).toBeVisible();
  },
};
