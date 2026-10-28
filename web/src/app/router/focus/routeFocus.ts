import { announce } from "~/shared/lib/a11y";

const HEADING_SELECTOR = "main h1";
/** Сколько ждём заголовок новой страницы: дольше — страница без h1, остаётся объявление заголовка. */
const HEADING_TIMEOUT_MS = 1000;

type NavigationInfo = { fromLocation?: unknown; pathChanged: boolean };

/**
 * Фокус переносится только при смене pathname: смена search/hash (фильтры, вкладка, `?selected=`) его не
 * трогает (WCAG 3.2.2), а на первой загрузке документа фокус остаётся у skip-link.
 */
export function shouldMoveFocus(info: NavigationInfo): boolean {
  return info.fromLocation !== undefined && info.pathChanged;
}

export function findHeading(root: ParentNode): HTMLElement | null {
  return root.querySelector<HTMLElement>(HEADING_SELECTOR);
}

/** Фокус на заголовок без прокрутки; tabindex=-1 ставим сами, если заголовок его не имеет (ProblemState). */
function focusHeading(heading: HTMLElement): void {
  if (!heading.hasAttribute("tabindex")) {
    heading.tabIndex = -1;
  }
  heading.focus({ preventScroll: true });
}

/**
 * Ждёт h1 новой страницы и переносит на него фокус. `stale` — заголовок страницы, с которой ушли: пока React
 * не заменил его, фокус на него бы потерялся вместе с узлом. Скелетон h1 не содержит, фокус не получает.
 * Возвращает отмену. Если заголовка так и не появилось, заголовок объявляется через live-регион.
 */
export function focusHeadingWhenReady(root: HTMLElement, stale: HTMLElement | null): () => void {
  let done = false;
  const observer = new MutationObserver(() => attempt());

  function finish(): void {
    done = true;
    observer.disconnect();
    clearTimeout(timer);
  }

  function attempt(): void {
    const heading = findHeading(root);
    if (!done && heading !== null && heading !== stale) {
      finish();
      focusHeading(heading);
    }
  }

  const timer = setTimeout(() => {
    finish();
    const heading = findHeading(root);
    // Тот же узел, что до навигации, — заголовок ушедшей страницы: фокус на нём ничего не объявит.
    if (heading === null || heading === stale) {
      announce(document.title);
    } else {
      focusHeading(heading);
    }
  }, HEADING_TIMEOUT_MS);

  observer.observe(root, { childList: true, subtree: true });
  attempt();
  return finish;
}

/** Фокус сейчас внутри `<main>` (в блоке ошибки или на странице), а не в сайдбаре или topbar. */
export function isFocusInMain(): boolean {
  const main = document.querySelector("main");
  return main?.contains(document.activeElement) === true;
}

/** Фокус на h1 того, что показано сейчас. */
export function focusMainHeading(): void {
  const heading = findHeading(document.body);
  if (heading !== null) {
    focusHeading(heading);
  }
}
