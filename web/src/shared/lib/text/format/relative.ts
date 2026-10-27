const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const JUST_NOW = 10 * SECOND;
const NO_VALUE = "—";

const sameYear = new Intl.DateTimeFormat("ru-RU", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});
const otherYear = new Intl.DateTimeFormat("ru-RU", {
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});
const full = new Intl.DateTimeFormat("ru-RU", { dateStyle: "long", timeStyle: "medium" });

type Timestamp = string | number | Date;

function toMillis(ts: Timestamp): number {
  return ts instanceof Date ? ts.getTime() : new Date(ts).getTime();
}

/**
 * Относительное время: до 10 с «только что», до минуты «N с назад», до часа «N мин назад», до суток «N ч назад»,
 * дальше дата «20 окт., 14:03» (для другого года — с годом). Будущее показывается как «только что».
 */
export function formatRelative(ts: Timestamp, now: number): string {
  const time = toMillis(ts);
  if (Number.isNaN(time)) {
    return NO_VALUE;
  }
  const diff = now - time;
  if (diff < JUST_NOW) {
    return "только что";
  }
  if (diff < MINUTE) {
    return `${Math.floor(diff / SECOND)} с назад`;
  }
  if (diff < HOUR) {
    return `${Math.floor(diff / MINUTE)} мин назад`;
  }
  if (diff < DAY) {
    return `${Math.floor(diff / HOUR)} ч назад`;
  }
  const date = new Date(time);
  return (date.getFullYear() === new Date(now).getFullYear() ? sameYear : otherYear).format(date);
}

/** Полная дата и время для `title`: «20 октября 2026 г. в 14:03:07». */
export function formatDateTime(ts: Timestamp): string {
  const time = toMillis(ts);
  return Number.isNaN(time) ? NO_VALUE : full.format(new Date(time));
}
