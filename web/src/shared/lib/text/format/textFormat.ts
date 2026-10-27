type Timestamp = string | number | Date;

type CountValue = {
  count: number;
  /** Сервер ограничил подсчёт: реальное число не меньше `count`. */
  capped: boolean;
};

/** Форматтеры на одной локали. Чистые Intl-функции: Lingui и активная локаль им не нужны. */
export type TextFormat = {
  /** `48102` → «48 102» (ru) / «48,102» (en). */
  number(value: number, options?: Intl.NumberFormatOptions): string;
  /** Доля 0..1 → «94,2 %» (ru) / «94.2%» (en), один знак после запятой. */
  percent(ratio: number): string;
  /** Размер в байтах → «18,4 ТБ». Ступень 1024; пробел перед единицей неразрывный. */
  bytes(bytes: number): string;
  /** `{1000, capped}` → «1 000+»: усечённый счётчик нельзя показывать как точный. */
  count(value: CountValue): string;
  /**
   * Относительное время через `Intl.RelativeTimeFormat` («3 мин. назад», «вчера»); старше двух суток — дата.
   * Будущее показывается как «сейчас».
   */
  relative(ts: Timestamp, now: number): string;
  /** Полная дата и время для `title`: «20 октября 2026 г. в 14:03:07». */
  dateTime(ts: Timestamp): string;
  /** Короткие дата и время для подсказок графиков: «9 окт., 03:00»; год не показывается. */
  dateTimeShort(ts: Timestamp): string;
};

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const JUST_NOW = 10 * SECOND;
const NO_VALUE = "—";
const NBSP = " ";
const BYTE_UNITS = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte", "petabyte"] as const;
const BYTES_STEP = 1024;

function toMillis(ts: Timestamp): number {
  return ts instanceof Date ? ts.getTime() : new Date(ts).getTime();
}

function build(locale: string): TextFormat {
  const numberFormats = new Map<string, Intl.NumberFormat>();
  const numberFormat = (options: Intl.NumberFormatOptions): Intl.NumberFormat => {
    const key = JSON.stringify(options);
    let cached = numberFormats.get(key);
    if (cached === undefined) {
      cached = new Intl.NumberFormat(locale, options);
      numberFormats.set(key, cached);
    }
    return cached;
  };
  const relative = new Intl.RelativeTimeFormat(locale, { numeric: "auto", style: "short" });
  const sameYear = new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
  const otherYear = new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
  const full = new Intl.DateTimeFormat(locale, { dateStyle: "long", timeStyle: "medium" });

  const number = (value: number, options: Intl.NumberFormatOptions = {}): string =>
    numberFormat(options).format(value);

  return {
    number,
    percent: (ratio) =>
      numberFormat({ style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(
        ratio,
      ),
    bytes(bytes) {
      let value = bytes;
      let unit = 0;
      while (Math.abs(value) >= BYTES_STEP && unit < BYTE_UNITS.length - 1) {
        value /= BYTES_STEP;
        unit += 1;
      }
      return numberFormat({
        style: "unit",
        unit: BYTE_UNITS[unit] ?? "byte",
        unitDisplay: "short",
        maximumFractionDigits: unit === 0 ? 0 : 1,
      })
        .format(value)
        .replaceAll(" ", NBSP);
    },
    count: ({ count, capped }) => (capped ? `${number(count)}+` : number(count)),
    relative(ts, now) {
      const time = toMillis(ts);
      if (Number.isNaN(time)) {
        return NO_VALUE;
      }
      const diff = now - time;
      if (diff < JUST_NOW) {
        return relative.format(0, "second");
      }
      if (diff < MINUTE) {
        return relative.format(-Math.floor(diff / SECOND), "second");
      }
      if (diff < HOUR) {
        return relative.format(-Math.floor(diff / MINUTE), "minute");
      }
      if (diff < DAY) {
        return relative.format(-Math.floor(diff / HOUR), "hour");
      }
      if (diff < 2 * DAY) {
        return relative.format(-1, "day");
      }
      const date = new Date(time);
      return (date.getFullYear() === new Date(now).getFullYear() ? sameYear : otherYear).format(
        date,
      );
    },
    dateTime(ts) {
      const time = toMillis(ts);
      return Number.isNaN(time) ? NO_VALUE : full.format(new Date(time));
    },
    dateTimeShort(ts) {
      const time = toMillis(ts);
      return Number.isNaN(time) ? NO_VALUE : sameYear.format(new Date(time));
    },
  };
}

const cache = new Map<string, TextFormat>();

/** Форматтеры для локали; объекты Intl кэшируются на локаль. */
export function textFormat(locale: string): TextFormat {
  let cached = cache.get(locale);
  if (cached === undefined) {
    cached = build(locale);
    cache.set(locale, cached);
  }
  return cached;
}
