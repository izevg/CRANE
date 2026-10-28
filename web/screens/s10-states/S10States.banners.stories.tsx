import { msg } from "@lingui/core/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import { showToast } from "~/shared/lib/state";
import type { Banner } from "~/widgets/app-shell";
import { fitsViewport } from "../frame/fit";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, mobile320, screenParameters } from "../frame/screen";
import { PackagesBackdrop } from "./PackagesBackdrop";
import { Frame } from "./S10Frame";
import { BODY, dark, MORE_BANNERS, phone, REQUEST_ID } from "./S10Shared";

const meta = {
  title: "Экраны/S10 Состояния и оверлеи",
  component: ScreenFrame,
  parameters: screenParameters,
  globals: desktop,
  args: { active: "packages", breadcrumbs: { trail: [], current: "" }, children: null },
} satisfies Meta<typeof ScreenFrame>;

export default meta;
type Story = StoryObj<typeof meta>;

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

/** Топбар не шире окна на телефоне (WCAG 1.4.10): ширины 390 и 320, обе локали. */
export const Fit390: Story = {
  ...banners,
  name: "Телефон 390: топбар помещается",
  globals: mobile,
  play: fitsViewport,
};

export const Fit320: Story = {
  ...banners,
  name: "Телефон 320: топбар помещается",
  globals: mobile320,
  play: fitsViewport,
};

export const Fit390English: Story = {
  ...banners,
  name: "Телефон 390 EN: топбар помещается",
  globals: { ...mobile, locale: "en" },
  play: fitsViewport,
};

export const Fit320English: Story = {
  ...banners,
  name: "Телефон 320 EN: топбар помещается",
  globals: { ...mobile320, locale: "en" },
  play: fitsViewport,
};
