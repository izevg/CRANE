import { type ReactNode, useCallback, useEffect } from "react";
import { LiveRegion } from "~/shared/lib/a11y";
import { I18nProvider, i18n } from "~/shared/lib/i18n";
import { type LinkProps, LinkProvider, Toaster, TooltipProvider } from "~/shared/ui/primitives";

/**
 * Ссылка витрины: обычный `<a>`. Переход гасит `PreventNavigation`, иначе iframe ушёл бы со страницы.
 * Имя со строчной буквы: это значение для `LinkProvider`, а не компонент, который экспортируется.
 */
function storyLink(props: LinkProps) {
  return <a {...props} />;
}

function PreventNavigation() {
  const handleClick = useCallback((event: globalThis.MouseEvent) => {
    if (event.target instanceof Element && event.target.closest("a[href]")) {
      event.preventDefault();
    }
  }, []);
  useEffect(() => {
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, [handleClick]);
  return null;
}

/** Всё, что приложение монтирует в корне: ссылки, подсказки, live-регион и тосты. */
export function StoryShell({ children }: { children: ReactNode }) {
  return (
    <I18nProvider i18n={i18n}>
      <LinkProvider component={storyLink}>
        <TooltipProvider>
          {children}
          <LiveRegion />
          <Toaster />
          <PreventNavigation />
        </TooltipProvider>
      </LinkProvider>
    </I18nProvider>
  );
}
