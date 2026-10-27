import { expect, waitFor, within } from "storybook/test";

const AVATAR = /аккаунт|account/;
const BELL = /Уведомления|Notifications/;
const TOLERANCE = 0.5;
const SHOWN = 8;

/** Первые элементы, выступающие за правый край окна, — в сообщении ошибки, чтобы не искать виновника вручную. */
function offenders(body: HTMLElement): string {
  return [...body.querySelectorAll("*")]
    .filter((element) => element.getBoundingClientRect().right > window.innerWidth + TOLERANCE)
    .slice(0, SHOWN)
    .map(
      (element) =>
        `${element.tagName.toLowerCase()} right=${Math.round(element.getBoundingClientRect().right)}`,
    )
    .join("; ");
}

/**
 * Регрессия WCAG 1.4.10 для экранов на телефоне: страница не шире окна, а колокольчик и аккаунт целиком в области
 * просмотра. Годится как `play` мобильной story; ширину окна задают viewport-глобалы (`mobile`, `mobile320`).
 * Кнопки ищутся и под модальной панелью (`hidden: true`): она делает страницу за собой инертной.
 */
export async function fitsViewport({ canvasElement }: { canvasElement: HTMLElement }) {
  const { documentElement, body } = canvasElement.ownerDocument;
  await waitFor(() => {
    expect(documentElement.scrollWidth, offenders(body)).toBeLessThanOrEqual(window.innerWidth);
  });
  const canvas = within(canvasElement);
  const avatar = await canvas.findByRole("button", { name: AVATAR, hidden: true });
  const bell = await canvas.findByRole("button", { name: BELL, hidden: true });
  await expect(avatar.getBoundingClientRect().right).toBeLessThanOrEqual(window.innerWidth);
  await expect(bell.getBoundingClientRect().right).toBeLessThanOrEqual(window.innerWidth);
}
