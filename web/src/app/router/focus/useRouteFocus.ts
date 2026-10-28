import { useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { findHeading, focusHeadingWhenReady, shouldMoveFocus } from "./routeFocus";

/** Подписка на завершение навигации: фокус на h1 новой страницы при смене pathname. */
export function useRouteFocus(): void {
  const router = useRouter();
  useEffect(() => {
    let stale: HTMLElement | null = null;
    let cancel = (): void => undefined;
    const offBefore = router.subscribe("onBeforeNavigate", (event) => {
      if (event.pathChanged) {
        stale = findHeading(document.body);
      }
    });
    const offResolved = router.subscribe("onResolved", (event) => {
      cancel();
      if (shouldMoveFocus(event)) {
        cancel = focusHeadingWhenReady(document.body, stale);
      }
    });
    return () => {
      offBefore();
      offResolved();
      cancel();
    };
  }, [router]);
}
