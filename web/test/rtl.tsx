import { i18n } from "@lingui/core";
import { I18nProvider } from "@lingui/react";
import { render as baseRender, renderHook as baseRenderHook } from "@testing-library/react/pure";
import type { ReactNode } from "react";

export * from "@testing-library/react/pure";

/** Компонентам нужен `I18nProvider` (useLingui). Свой `wrapper` в options заменяет этот и берёт провайдер на себя. */
function I18nWrapper({ children }: { children: ReactNode }) {
  return <I18nProvider i18n={i18n}>{children}</I18nProvider>;
}

export function render(
  ...[ui, options]: Parameters<typeof baseRender>
): ReturnType<typeof baseRender> {
  return baseRender(ui, { wrapper: I18nWrapper, ...options });
}

export function renderHook(
  ...[callback, options]: Parameters<typeof baseRenderHook>
): ReturnType<typeof baseRenderHook> {
  return baseRenderHook(callback, { wrapper: I18nWrapper, ...options });
}
