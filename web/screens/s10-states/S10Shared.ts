import { msg } from "@lingui/core/macro";
import { within } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import { desktop, mobile } from "../frame/screen";

export const BELL = /^Уведомления/;
export const MORE_BANNERS = /Ещё 2/;
export const BODY = () => within(document.body);
export const REQUEST_ID = "req_01JBX7Q4M2K9T5V8N3PZ6H1WRD";
export function home() {
  return {
    label: i18n._(msg`На главную`),
    run: { href: "/" },
  } as const;
}

/** Тёмный и мобильный варианты строятся из светлого desktop без дублирования. */
export const dark = <T extends { globals?: object }>(story: T): T => ({
  ...story,
  globals: { ...desktop, theme: "dark" },
});
export const phone = <T extends { globals?: object }>(story: T): T => ({
  ...story,
  globals: { ...mobile },
});
